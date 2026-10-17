//! Offline format converter from original PC assets to native Android runtime assets.
//!
//! Converts legacy EA `GEOMETRY.BIN` containers into cache-aligned, GPU-ready `.onfs-model` files.
//! Supports both single-file conversion and full garage batch conversion.

use clap::Parser;
use open_nfs_asset::error::GeometryError;
use open_nfs_asset::geometry::parse_geometry;
use open_nfs_asset_runtime::format::serialize_runtime_model;
use open_nfs_asset_runtime::model::{RuntimeBounds, RuntimeModel, RuntimePart, RuntimeSubmesh};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "open-nfs-converter",
    author = "Open-NFS Contributors",
    version,
    about = "Converts original PC Need for Speed Underground 2 assets to native Android engine format"
)]
struct Cli {
    /// Input path to a single PC asset (e.g., CARS/350Z/GEOMETRY.BIN)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output path for converted .onfs-model file (e.g., 350Z.onfs-model)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Batch directory containing vehicle subdirectories (e.g., game-assets/CARS/)
    #[arg(long)]
    batch_dir: Option<PathBuf>,

    /// Output directory for batch conversion (e.g., android-host/assets/models/)
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Optional target Android assets directory to bundle output (e.g., android-host/assets/models/)
    #[arg(short, long)]
    bundle_dir: Option<PathBuf>,

    /// Optional path to write a JSON conversion manifest
    #[arg(short, long)]
    manifest: Option<PathBuf>,

    /// Print detailed conversion logs
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone)]
struct ConversionStats {
    vehicle_name: String,
    source_path: PathBuf,
    source_sha256: String,
    source_bytes: usize,
    output_path: PathBuf,
    output_sha256: String,
    output_bytes: usize,
    parts_count: usize,
    total_vertices: usize,
    total_indices: usize,
}

fn main() -> ExitCode {
    let args = Cli::parse();

    if let Err(e) = run_converter(&args) {
        eprintln!("\x1b[1;31mError:\x1b[0m {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run_converter(args: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m=== Open-NFS Offline Asset Converter ===\x1b[0m");

    if let (Some(batch_dir), Some(output_dir)) = (&args.batch_dir, &args.output_dir) {
        run_batch_converter(args, batch_dir, output_dir)
    } else if let (Some(input), Some(output)) = (&args.input, &args.output) {
        run_single_converter(args, input, output)
    } else {
        Err("Please specify either single file mode (--input <FILE> --output <FILE>) or batch mode (--batch-dir <DIR> --output-dir <DIR>)".into())
    }
}

fn run_single_converter(
    args: &Cli,
    input: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Mode:   Single File");
    println!("Input:  {}", input.display());
    println!("Output: {}", output.display());

    let stats = convert_single_asset(
        input,
        output,
        args.bundle_dir.as_deref(),
        "Model",
        args.verbose,
    )?;

    if let Some(manifest_path) = &args.manifest {
        write_batch_manifest(manifest_path, std::slice::from_ref(&stats))?;
        println!("Manifest written to: {}", manifest_path.display());
    }

    let ratio = (stats.output_bytes as f64) / (stats.source_bytes as f64) * 100.0;
    println!("\x1b[1;32mConversion Successful!\x1b[0m");
    println!("------------------------------------------------------------");
    println!("Parts:          {}", stats.parts_count);
    println!("Vertices:       {}", stats.total_vertices);
    println!("Indices:        {}", stats.total_indices);
    println!(
        "Original Size:  {:.2} MiB ({} bytes)",
        stats.source_bytes as f64 / 1_048_576.0,
        stats.source_bytes
    );
    println!(
        "Runtime Size:   {:.2} MiB ({} bytes) [{:.1}% of original]",
        stats.output_bytes as f64 / 1_048_576.0,
        stats.output_bytes,
        ratio
    );
    println!("Source SHA-256: {}", stats.source_sha256);
    println!("Output SHA-256: {}", stats.output_sha256);
    println!("------------------------------------------------------------");

    Ok(())
}

fn run_batch_converter(
    args: &Cli,
    batch_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Mode:       Batch Conversion");
    println!("Batch Dir:  {}", batch_dir.display());
    println!("Output Dir: {}", output_dir.display());

    if !batch_dir.exists() {
        return Err(format!("Batch directory does not exist: {}", batch_dir.display()).into());
    }

    std::fs::create_dir_all(output_dir)?;

    let mut entries: Vec<PathBuf> = std::fs::read_dir(batch_dir)?
        .filter_map(|e| e.ok().map(|d| d.path()))
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(entries.len().max(1));
    println!("Threads:    {}", num_threads);

    enum BatchItemResult {
        Success(ConversionStats),
        Skipped(String),
        Failure(String, String),
    }

    let chunk_size = entries.len().div_ceil(num_threads);
    let chunks: Vec<_> = entries.chunks(chunk_size.max(1)).collect();

    let results: Vec<BatchItemResult> = std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            handles.push(s.spawn(move || {
                let mut local = Vec::with_capacity(chunk.len());
                for dir in chunk {
                    let vehicle_name = dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("UNKNOWN");

                    let geometry_path = dir.join("GEOMETRY.BIN");
                    if !geometry_path.exists() {
                        continue;
                    }

                    let out_file = output_dir.join(format!("{}.onfs-model", vehicle_name));
                    match convert_single_asset(
                        &geometry_path,
                        &out_file,
                        args.bundle_dir.as_deref(),
                        vehicle_name,
                        args.verbose,
                    ) {
                        Ok(stats) => local.push(BatchItemResult::Success(stats)),
                        Err(e) => {
                            if let Some(geom_err) = e.downcast_ref::<GeometryError>() {
                                if matches!(geom_err, GeometryError::MissingMandatoryChunk(_)) {
                                    local.push(BatchItemResult::Skipped(vehicle_name.to_string()));
                                    continue;
                                }
                            }
                            local.push(BatchItemResult::Failure(
                                vehicle_name.to_string(),
                                e.to_string(),
                            ));
                        }
                    }
                }
                local
            }));
        }

        let mut all = Vec::with_capacity(entries.len());
        for h in handles {
            if let Ok(res) = h.join() {
                all.extend(res);
            }
        }
        all
    });

    let mut successful: Vec<ConversionStats> = Vec::new();
    let mut skipped: usize = 0;

    for item in results {
        match item {
            BatchItemResult::Success(stats) => {
                println!(
                    "  \x1b[32m✓\x1b[0m \x1b[1m{:<16}\x1b[0m {:>4} parts | {:>6} verts | {:>6} idx | {:>5.2} MiB",
                    stats.vehicle_name,
                    stats.parts_count,
                    stats.total_vertices,
                    stats.total_indices,
                    stats.output_bytes as f64 / 1_048_576.0
                );
                successful.push(stats);
            }
            BatchItemResult::Skipped(vehicle_name) => {
                if args.verbose {
                    println!(
                        "  \x1b[33m-\x1b[0m {:<16} [Skipped: non-car accessory container]",
                        vehicle_name
                    );
                }
                skipped += 1;
            }
            BatchItemResult::Failure(vehicle_name, err_msg) => {
                eprintln!("  \x1b[31m✗\x1b[0m {:<16} Error: {}", vehicle_name, err_msg);
            }
        }
    }

    successful.sort_by(|a, b| a.vehicle_name.cmp(&b.vehicle_name));

    if let Some(manifest_path) = &args.manifest {
        write_batch_manifest(manifest_path, &successful)?;
        println!(
            "\nMaster batch manifest written to: {}",
            manifest_path.display()
        );
    }

    let total_orig_bytes: usize = successful.iter().map(|s| s.source_bytes).sum();
    let total_out_bytes: usize = successful.iter().map(|s| s.output_bytes).sum();
    let total_verts: usize = successful.iter().map(|s| s.total_vertices).sum();
    let total_indices: usize = successful.iter().map(|s| s.total_indices).sum();
    let total_parts: usize = successful.iter().map(|s| s.parts_count).sum();
    let ratio = (total_out_bytes as f64) / (total_orig_bytes as f64) * 100.0;

    println!("\n\x1b[1;32m=== Batch Conversion Summary ===\x1b[0m");
    println!("------------------------------------------------------------");
    println!("Vehicles Converted:  {}", successful.len());
    println!("Accessories Skipped: {}", skipped);
    println!("Total Parts:         {}", total_parts);
    println!("Total Vertices:      {}", total_verts);
    println!("Total Indices:       {}", total_indices);
    println!(
        "Original Total Size: {:.2} MiB ({} bytes)",
        total_orig_bytes as f64 / 1_048_576.0,
        total_orig_bytes
    );
    println!(
        "Runtime Total Size:  {:.2} MiB ({} bytes) [{:.1}% of original]",
        total_out_bytes as f64 / 1_048_576.0,
        total_out_bytes,
        ratio
    );
    println!("Storage Reduction:   {:.1}%", 100.0 - ratio);
    println!("------------------------------------------------------------");

    Ok(())
}

fn convert_single_asset(
    input: &Path,
    output: &Path,
    bundle_dir: Option<&Path>,
    vehicle_name: &str,
    verbose: bool,
) -> Result<ConversionStats, Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("Input file does not exist: {}", input.display()).into());
    }

    // 1. Read input bytes & calculate source SHA-256
    let input_bytes = std::fs::read(input)?;
    let source_sha256 = compute_sha256(&input_bytes);
    let source_hash_hex = hex_digest(&source_sha256);

    if verbose {
        println!("  [{}]: Source SHA-256 = {}", vehicle_name, source_hash_hex);
    }

    // 2. Parse original geometry
    let archive = parse_geometry(&input_bytes)?;

    // 3. Convert to RuntimeModel
    let mut runtime_parts = Vec::with_capacity(archive.parts.len());
    for part in archive.parts {
        let submeshes: Vec<RuntimeSubmesh> = part
            .submeshes
            .into_iter()
            .map(|sm| RuntimeSubmesh {
                material_id: sm.material_id,
                index_offset: sm.index_offset,
                index_count: sm.index_count,
                shader_flags: sm.shader_flags,
                bounds: RuntimeBounds {
                    min: sm.bbox_min,
                    max: sm.bbox_max,
                },
            })
            .collect();

        runtime_parts.push(RuntimePart {
            name: part.header.name,
            lod_level: part.header.lod_level,
            part_hash: part.header.part_hash,
            bounds: RuntimeBounds {
                min: part.header.bbox_min,
                max: part.header.bbox_max,
            },
            vertices: part.vertices,
            indices: part.indices,
            submeshes,
        });
    }

    let model = RuntimeModel {
        source_sha256,
        parts: runtime_parts,
    };

    let total_vertices = model.total_vertices();
    let total_indices = model.total_indices();
    let parts_count = model.parts.len();

    // 4. Serialize to .onfs-model format
    let output_bytes = serialize_runtime_model(&model);
    let output_sha256 = compute_sha256(&output_bytes);
    let output_hash_hex = hex_digest(&output_sha256);

    // 5. Ensure output directory exists and write file
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(output, &output_bytes)?;

    // 6. Handle optional bundle directory
    if let Some(b_dir) = bundle_dir {
        std::fs::create_dir_all(b_dir)?;
        let file_name = output
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("model.onfs-model"));
        let target_path = b_dir.join(file_name);
        std::fs::write(&target_path, &output_bytes)?;
    }

    Ok(ConversionStats {
        vehicle_name: vehicle_name.to_string(),
        source_path: input.to_path_buf(),
        source_sha256: source_hash_hex,
        source_bytes: input_bytes.len(),
        output_path: output.to_path_buf(),
        output_sha256: output_hash_hex,
        output_bytes: output_bytes.len(),
        parts_count,
        total_vertices,
        total_indices,
    })
}

fn write_batch_manifest(path: &Path, list: &[ConversionStats]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }

    let total_orig: usize = list.iter().map(|s| s.source_bytes).sum();
    let total_out: usize = list.iter().map(|s| s.output_bytes).sum();
    let total_verts: usize = list.iter().map(|s| s.total_vertices).sum();
    let total_indices: usize = list.iter().map(|s| s.total_indices).sum();

    let mut json = format!(
        "{{\n  \"total_vehicles\": {},\n  \"total_source_bytes\": {},\n  \"total_runtime_bytes\": {},\n  \"total_vertices\": {},\n  \"total_indices\": {},\n  \"vehicles\": [\n",
        list.len(), total_orig, total_out, total_verts, total_indices
    );

    for (i, it) in list.iter().enumerate() {
        let comma = if i + 1 < list.len() { "," } else { "" };
        let _ = write!(
            json,
            "    {{\n      \"name\": \"{}\",\n      \"source_path\": \"{}\",\n      \"source_sha256\": \"{}\",\n      \"source_bytes\": {},\n      \"output_path\": \"{}\",\n      \"output_sha256\": \"{}\",\n      \"output_bytes\": {},\n      \"parts_count\": {},\n      \"total_vertices\": {},\n      \"total_indices\": {}\n    }}{}\n",
            it.vehicle_name,
            it.source_path.display(),
            it.source_sha256,
            it.source_bytes,
            it.output_path.display(),
            it.output_sha256,
            it.output_bytes,
            it.parts_count,
            it.total_vertices,
            it.total_indices,
            comma
        );
    }

    json.push_str("  ]\n}\n");
    std::fs::write(path, json)
}

fn compute_sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

fn hex_digest(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

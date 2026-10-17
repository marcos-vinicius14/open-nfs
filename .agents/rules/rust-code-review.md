Metadata
trigger
model_decision
description
Aplicar revisão rigorosa em código Rust ao analisar alterações, pull requests, refatorações e implementações em arquivos .rs ou Cargo.toml.
Rust Code Review — Regra do agente
Objetivo
Revisar código Rust com foco em correção, segurança, robustez, manutenção e desempenho. Priorizar defeitos demonstráveis sobre preferências de estilo. Aplicar a regra durante code reviews e, preventivamente, ao modificar Rust. Não transformar uma revisão em refatoração não solicitada.

Procedimento obrigatório
Identificar escopo da alteração, contratos, invariantes, pontos de entrada, dependências, features e versão de Rust (MSRV).
Ler implementação, chamadores e testes relevantes antes de concluir que existe um defeito.
Considerar entradas inválidas, falhas externas, concorrência, cancelamento, efeitos colaterais e limites de recursos.
Verificar cada categoria abaixo quando aplicável. Não reportar hipóteses genéricas sem um caminho de falha plausível.
Executar verificações disponíveis e registrar resultados reais. Não afirmar que executou comandos não executados.
Apresentar achados ordenados por severidade com referência precisa a arquivo/linha, cenário, impacto e correção objetiva.
1. Erros, panics e contratos
Examinar unwrap, expect, panic!, todo!, unimplemented!, unreachable!, indexação (vec[i]) e operações que podem estourar limites. São aceitáveis apenas quando a invariante está demonstrada ou quando panic é deliberadamente parte do contrato.
Garantir tratamento ou propagação de Result e Option; sinalizar resultados descartados (let _ = ...) quando o erro for relevante.
Verificar a distinção entre erro recuperável, violação de invariante e falha fatal.
Preservar causa/contexto dos erros (source, #[from], map_err, anyhow::Context), sem expor segredos em mensagens.
Analisar modelagem de erros com thiserror: mensagens #[error(...)], campos referenciados, #[source] e #[from]. São atributos de biblioteca, não da linguagem.
Verificar coerência entre documentação, tipos de retorno, contratos públicos e comportamento sob falhas.
Verificar ausência de truncamento silencioso, coerções numéricas indevidas e uso de aritmética com overflow sem semântica intencional (checked_*, saturating_*, wrapping_*, conforme contrato).
2. Ownership, lifetimes e recursos
Procurar clone, to_owned, to_string, collect e alocações em loops sem necessidade comprovada; não recomendar removê-los sem avaliar ownership e legibilidade.
Verificar preferências apropriadas por &str/&[T] em APIs somente de leitura, sem impor mudanças em interfaces que precisam de ownership.
Examinar uso artificial de 'static, Box::leak, mem::forget, referências cíclicas com Rc/Arc e possíveis retenções de memória ou recursos.
Conferir liberação de locks, arquivos, conexões e recursos em todos os caminhos, inclusive erro e cancelamento.
Revisar RefCell/Mutex em relação a panics, contenção, granularidade e necessidade real de estado compartilhado.
3. Concorrência e async
Verificar chamadas bloqueantes dentro de async (I/O síncrona, thread::sleep, computação pesada), seleção apropriada de spawn_blocking e custo do pool.
Revisar locks mantidos durante .await, ordem de aquisição, deadlocks e starvation. Não classificar todo std::sync::Mutex em async como erro: considerar duração do lock e ausência de .await durante sua posse.
Verificar tratamento de JoinHandle, falhas de tarefas, shutdown, cancelamento e operações com efeitos parciais.
Revisar tokio::select!, join!, timeouts, backpressure, canais sem limite e propagação de falhas.
Examinar consistência de estados compartilhados, transições atômicas e ordenação de memória (Atomic*).
Avaliar idempotência e semântica de retries, especialmente quando houver operações externas.
4. Segurança e unsafe
Tratar unsafe, unsafe impl Send/Sync, ponteiros brutos, FFI, transmute, MaybeUninit e operações *_unchecked como áreas de revisão prioritária.
Exigir justificativa SAFETY próxima ao bloco/implementação unsafe, com invariantes concretas e verificáveis; conferir validade de ponteiros, alinhamento, inicialização, aliases, lifetimes e contratos FFI.
Validar entradas não confiáveis: caminhos, tamanhos, contadores, payloads, serialização, parsing, Unicode/ASCII e limites de memória/CPU.
Examinar construção de comandos (std::process::Command), argumentos e fronteiras com shell; nunca presumir shell injection se não há shell envolvido.
Procurar segredos ou dados pessoais em logs, erros e Debug; verificar permissões e configuração segura.
Considerar riscos de dependências e features habilitadas, com atenção a advisories quando houver ferramenta/dados disponíveis.
5. Atributos e macros
Revisar quando presentes, sem tratar presença isolada como defeito:

#[derive(...)]: semântica de Clone, Copy, Debug, Eq, Hash, Default e traits derivadas.
#[must_use]: resultados importantes descartados; não recomendar aplicação indiscriminada.
#[allow(...)], #[expect(...)], #[deny(...)]: supressões desnecessárias, justificativas e abrangência.
#[cfg(...)], #[cfg_attr(...)]: combinações de features, código específico de plataforma e builds alternativos.
#[repr(...)]: contratos de layout e FFI.
#[inline], #[deprecated] e atributos #[unsafe(...)]: uso e consequências reais.
#[serde(...)]: defaults, deny_unknown_fields, renomeações, campos opcionais, compatibilidade de contratos e validação posterior à desserialização.
#[tokio::main] e #[tokio::test]: runtime e natureza dos testes.
Macros de crates externas: confirmar semântica na dependência e versão utilizada antes de reportar.
6. Desempenho
Avaliar complexidade algorítmica, cópias/alocações, iterações repetidas, consumo de memória e comportamento com entradas grandes.
Revisar escolha de coleções (Vec, HashSet, HashMap, BTreeMap) conforme operações reais, sem trocar estruturas apenas por preferência.
Identificar retenção desnecessária, buffers ilimitados, concorrência descontrolada e parsing que aceita tamanhos abusivos.
Requerer evidência, perfil ou cenário concreto antes de propor micro-otimizações.
7. Arquitetura e legibilidade
Verificar separação de regras de negócio e infraestrutura, responsabilidades coesas e efeitos colaterais explícitos.
Examinar visibilidade (pub), contratos estáveis, nomes expressivos, abstrações úteis e uso justificado de traits/generics.
Preferir tipos que tornem estados inválidos irrepresentáveis quando isso simplificar o domínio; verificar enums e state machines.
Evitar duplicação de regras críticas e APIs excessivamente genéricas; não exigir padrões arquiteturais sem benefício concreto.
8. Testes
Cobrar testes significativos para regras de negócio, limites, entradas inválidas, erros externos e regressões.
Para async/concorrência, testar timeouts, cancelamento, falha de tarefas e shutdown quando aplicável.
Considerar property-based testing e fuzzing para parsers e componentes com invariantes complexas.
Rejeitar testes que apenas reafirmam a implementação ou somente verificam mocks sem comportamento observável.
Não usar cobertura percentual isolada como critério de qualidade.
Validações automáticas
Quando o projeto e as ferramentas estiverem disponíveis, executar:

cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
--all-features pode falhar legitimamente se houver features mutuamente exclusivas; nesse caso, testar combinações válidas explicitamente.
Respeitar a toolchain e o MSRV do repositório.
Se cargo-audit estiver instalado, usar cargo audit para dependências; não instalá-lo automaticamente.
Se houver unsafe complexo, considerar Miri quando suportado e viável.
Não alterar configuração de lints, dependências ou código sem solicitação.
Severidade dos achados
Crítica: exploração plausível, corrupção de dados, UB alcançável ou exposição grave de segredos.
Alta: comportamento incorreto reproduzível, panic em entrada não confiável, deadlock provável, perda de dados ou falha de segurança relevante.
Média: falha em cenário de borda plausível, perda de diagnóstico importante, problema de recursos/concorrência ou manutenção com impacto demonstrável.
Baixa: melhoria concreta, localizada e não bloqueante.
Formato da resposta
Para cada achado, apresentar:

[Alta] Título objetivo — src/modulo.rs:42
Problema: condição concreta que ativa o defeito.
Impacto: consequência observável.
Correção: mudança mínima recomendada.
Depois dos achados, registrar: comandos executados e respectivos resultados; verificações não executadas e motivo; riscos residuais pertinentes. Se não houver achados, declarar "Nenhum problema demonstrável encontrado no escopo revisado", sem alegar garantia de ausência de defeitos.

Restrições
Nunca inventar falhas, números de linha, resultados de ferramenta ou garantias de segurança.
Não bloquear PR por preferência subjetiva, clone isolado, unwrap em teste ou uso justificado de unsafe.
Não recomendar mudanças desnecessárias nem refatorações extensas quando existe correção localizada.
Não mascarar bugs com #[allow], unwrap, defaults artificiais ou tratamento silencioso de erros.
Quando houver incerteza, descrever a premissa e como validá-la; não afirmar como defeito confirmado.

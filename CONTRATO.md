# CONTRATO — Jim 0.1.0 (sessão C0)

Jim é uma cópia enxuta, em Rust + Python, do Gauntlet Loops `v5-alpha` (repositório privado;
tag `v5-alpha`, commit `f761213`). Ideia central: **o loop obriga o agente a provar.** Um PASS só
sai com evidência reproduzida fora do agente; sem reprodução, o resultado é `UNPROVEN`, nunca PASS.

Documentos congelados por este contrato: este arquivo, `contract/jim_core.pyi` (interface),
`contract/SEMANTICA.md` (semântica exata), `contract/DIFERENCIAL.md` (testes contra a v5-alpha),
`Cargo.toml` e `pyproject.toml`.

## 1. Escopo da v0.1

### Entra

- **Gate** `gate_43` / `avaliar_criterio_43` com a semântica da v5-alpha, bit a bit, para entradas JSON.
- **Derivação de evidências** a partir de execuções observadas (parte de `tools/release.py`:
  `registros`, `avaliar_saida`, controles negativos), regras `esperado` e `todos_aprovados`.
- **Fecho**: inventário {caminho: SHA-256}, hash canônico (= `hash_arvore` da v5-alpha) e o
  documento `jim-fecho/1`, que substitui o manifesto de release.
- **Verificação** (equivalente enxuto de `tools/verificar.py`): fecho, contrato, árvore, streams,
  registros e gate recalculados; reprodução fora do agente.
- **Cadeia de hashes** JSONL (`cadeia.py`) para o registro local `.jim/cadeia.jsonl`.
- CLI `jim provar | verificar | cadeia verificar` e testes diferenciais por casos JSON.

### Fica fora (confirmado, com dois ajustes no fim)

| Item da v5-alpha | Motivo |
|---|---|
| `trabalho/` | histórico de lotes, validações e mutação; não é produto |
| specs antigas (`spec/4.2`–`4.5`) | a v0.1 congela `SEMANTICA.md`; as specs só servem de consulta privada |
| `releases/`, `INDICE.json`, manifestos, `tools/ancora.py`, `versao.py`, publicação de `release.py` | o Jim não lê nem publica releases antigas; o fecho faz o papel do manifesto |
| artefatos versionados (fechos, cadeias, saídas, `EVIDENCIAS.sha256`) | `.jim/` fica fora do git; a prova é refeita, não guardada |
| `index.html`, `tools/pagina.py` | página do projeto, sem papel na decisão |
| runner TypeScript (`runner/`, swarm, `trace_swarm`, `ponte_swarm`) | segunda implementação da cadeia e orquestração multiagente; no Jim há uma só (Rust) |
| juiz Jev/API (`juiz_jev`, `auditor`, `extrator_evidencia`, `comparador_numerico`, `jev_*`) | depende de modelo e API paga; a decisão do Jim não chama modelo |
| GPU/local (`gpu.py`, `local.py`, alocação, fronteira, Wilson) | escolher modelo não decide PASS |
| `core.gate` 4.2, parada e triagem | substituídos por `gate_43` e pelo veredito do Jim |
| `trace.py`, `historico*.py` | trace tipado 4.3–4.7; a 0.1 usa a cadeia crua com eventos simples |
| regras e campanhas de mutação (`sem_sobreviventes*`, `mutar*`, `checar_mutacao45`) | ferramenta de qualidade do Gauntlet, não do loop |
| aprovação humana no fluxo derivado | `gate_43` a suporta (e o diferencial a cobre), mas `jim-contrato/1` só gera evidência `execucao` |
| instrumento fora do fecho ou não-Python | o instrumento precisa estar no hash; comando arbitrário fica para depois |

Ajustes à lista proposta: (a) `tools/release.py` **entra em parte** (derivação, sem publicação),
porque é dele que sai o `adequado` que o gate lê — sem isso o gate decide sobre campos
autodeclarados; (b) as regras de mutação **saem**, para manter o núcleo pequeno (contrato que
as usa é recusado, J-K4).

## 2. Mapa de módulos

### Rust — crate `jim`, biblioteca `_core`, módulo `jim._core`: tudo que decide PASS/FAIL

| `src/` | Conteúdo | Origem na v5-alpha |
|---|---|---|
| `lib.rs` | ligação PyO3 (feature `python`), exceções, conversão J-D | — |
| `json.rs` | `json_estrito`, `json_canonico`, `iguais`, `repr_py` | `cadeia._carregar`, `release.iguais` |
| `hash.rs` | `sha256_hex`, `hash_linha`, `validar_caminho`, `hash_fecho`, `inventariar` | `cadeia.hash_linha`, `release.hash_arvore`, `caminho_seguro` |
| `gate.rs` | `avaliar_criterio_43`, `gate_43` | `gate43.py`, constantes de `core.py` |
| `registros.rs` | `validar_contrato`, `contrato_gate`, `ultima_linha_json`, `avaliar_saida`, `controles_aprovados`, `derivar_registros` | `tools/release.py` |
| `fecho.rs` | `montar_fecho`, `verificar_fecho`, `comparar_reproducao` | `tools/verificar.py` |
| `cadeia.rs` | `verificar_cadeia`, `eventos_cadeia`, `proxima_linha` | `cadeia.py` |

A lógica fica em módulos sem PyO3; `cargo test --no-default-features` a testa sem Python.

### Python — pacote `jim` em `python/jim`: orquestra, não decide

| Módulo | Conteúdo |
|---|---|
| `__init__.py` | `__version__` |
| `cli.py` | `main(argv=None) -> int`, subcomandos de §3 |
| `arquivos.py` | globs, leitura de bytes, cópia para diretório temporário, gravação atômica |
| `executor.py` | instrumentos e controles negativos em subprocesso; captura dos streams |
| `fluxo.py` | `provar` e `verificar` chamando `_core` |
| `registro.py` | `.jim/cadeia.jsonl` via `proxima_linha` |
| `_core.pyi` | cópia sem edição de `contract/jim_core.pyi` |

Regras do lado Python: nunca produz, altera ou interpreta veredito além de mapeá-lo para o
código de saída; nunca usa `json.loads` em dado que entra em decisão (usa `_core.json_estrito`);
nunca captura exceção de `_core` para seguir como sucesso; nunca escreve "PASS" que `_core`
não devolveu.

## 3. CLI (comportamento externo)

**`jim provar [--raiz DIR] [--contrato ARQ] [--fecho ARQ] [--cadeia ARQ]`**

1. Lê o contrato (padrão `RAIZ/jim.json`), `json_estrito` + `validar_contrato` com os arquivos
   resolvidos dos globs `arquivos` (glob recursivo relativo à raiz; só arquivos regulares; sem
   `__pycache__` e `.jim/`; ordenados, sem repetição; padrão sem arquivo = erro).
2. Copia esses arquivos para um diretório temporário novo; `executados = inventariar(cópia)`, que
   tem de ser igual a `inventariar(raiz)`.
3. Por critério, na ordem: primeiro os controles negativos (cada um numa cópia nova; texto grava
   UTF-8, `null` apaga; `alterado` = algum byte mudou), depois o comando
   `[sys.executable, "-B", *comando]` com cwd na cópia, `PYTHONDONTWRITEBYTECODE=1` e timeout
   `timeout_s` (padrão 900; estouro → exit 124, stdout `""`, stderr `"R-5.3: timeout do instrumento"`).
   Streams: bytes capturados, UTF-8 estrito, sem tradução de fim de linha; não-UTF-8 → erro (4).
4. `arquivos_depois` = `inventariar` dos caminhos que ainda são arquivos regulares na cópia;
   `contrato_sha256_depois` = SHA-256 do contrato relido.
5. `montar_fecho`; grava `dados` em `--fecho` (padrão `RAIZ/.jim/fecho.json`) de forma atômica;
   acrescenta à cadeia (padrão `RAIZ/.jim/cadeia.jsonl`) o evento
   `{"tipo": "fecho", "fecho_sha256", "candidato", "veredito"}`.
6. Imprime uma linha JSON canônica com `veredito`, `fecho_sha256`, `candidato`, `ancora_cadeia`.
   O veredito máximo de `provar` é `UNPROVEN`: quem rodou foi o agente.

**`jim verificar [--raiz DIR] [--contrato ARQ] [--fecho ARQ] [--ancora HEX] [--reproduzir]`**

1. Lê fecho e contrato do disco. O contrato vem do caminho dado por quem verifica, não do fecho.
2. `atuais` = `inventariar` dos globs do contrato resolvidos agora (`{}` se não resolver).
3. Sem `--reproduzir`: `verificar_fecho(..., reproducao=None)`; máximo `UNPROVEN`.
4. Com `--reproduzir`: só se o passo 3 deu `UNPROVEN` (nunca executa código de fecho que falhou
   na integridade): copia os caminhos de `fecho.arquivos` para temporário novo, mede
   `inventario_antes`, roda como em `provar`, mede `inventario_depois` e chama
   `verificar_fecho(..., reproducao)`. Só aqui pode sair PASS.
5. Imprime a `Verificacao` em JSON canônico.

**`jim cadeia verificar ARQ [--ancora N:HEX]`**: `verificar_cadeia`; sai 0 se íntegra e
ancorada, 3 se íntegra sem âncora, 1 nos demais. **`jim --version`**: `jim 0.1.0`.

**Códigos de saída**: 0 PASS · 1 FAIL · 2 UNKNOWN · 3 UNPROVEN · 4 erro (uso, I/O,
`EntradaInvalida`, `NaoImplementado`, falha interna). stdout tem só a linha JSON; diagnóstico vai
para stderr, começando pelo código do requisito. A CLI não imprime variáveis de ambiente.

## 4. Veredito: quatro valores

| Valor | Significa | Quem pode emitir |
|---|---|---|
| PASS | gate PASS sobre evidência íntegra **e** reprodução fora do agente igual ao gravado | só `verificar_fecho` com reprodução P7 |
| UNPROVEN | o gate passaria, mas falta reprodução (ou ela não cobre o fecho) | `montar_fecho`, `verificar_fecho` |
| FAIL | evidência contradiz o critério, ou algo diverge (hash, contrato, árvore, stream, reprodução) | todas |
| UNKNOWN | falta informação ou a entrada está malformada | todas |

`gate_43` continua com três valores (PASS/FAIL/UNKNOWN), como na v5-alpha; o PASS dele é
insumo, não veredito do Jim. Detalhe em `SEMANTICA.md` §5.4.

## 5. Decisão tools-N7: streams inteiros

**Contexto.** Na v5-alpha a execução grava só `stdout_sha256`, `stderr_sha256` e as últimas cinco
linhas do stderr; a `saida` que o gate lê é escrita por quem gravou e o verificador não a refaz
sem reexecutar. A validação da 4.5 achou 20 hashes de stdout sem vínculo por critério.

**Decisão.** O Jim **exige os streams inteiros** (stdout e stderr, texto UTF-8 exato) em toda
execução e todo controle negativo, no fecho e na reprodução. O núcleo confere
`sha256(stream) == hash gravado` e recalcula `saida` a partir do stdout (J-N7). Sem stream
inteiro, a execução não é aceita: fecho sem stream é malformado (UNKNOWN) e reprodução sem
stream é recusada.

**Motivo.** (1) A prova fica autocontida: o verificador refaz a derivação a partir do observado,
em vez de confiar num campo. (2) O Jim não lê releases antigas, então não há legado só-com-hash
para aceitar. (3) "stderr vazio" nos controles negativos vira fato conferível, não convenção de
campo ausente.

**Custos aceitos.** Fechos maiores; o que o instrumento imprime fica gravado (instrumento não
deve imprimir segredo; `.jim/` fica fora do git). A reprodução compara `exit_code` e a `saida`
derivada, não o stdout byte a byte (tempo e ordem de linhas variam); o stream inteiro continua
exigido dos dois lados.

## 6. Governança

| Arquivo | Dono |
|---|---|
| `CONTRATO.md`, `contract/*`, `Cargo.toml`, `pyproject.toml` | sessão de contrato (C0 e emendas) |
| `src/**` | sessão Rust |
| `python/jim/**`, `tests/**` (fora `tests/diferencial/casos/`) | lado Python |
| `tests/diferencial/casos/*` | só o gerador (`DIFERENCIAL.md`) |

- Interface ou semântica só mudam por emenda numerada (C0-1, C0-2…) registrada em §8; `.pyi`
  e `SEMANTICA.md` mudam no mesmo commit da emenda.
- Ambiguidade achada por quem implementa: registrar em `contract/QUESTOES.md` (pergunta, onde,
  proposta) e seguir a leitura mais restritiva — a que não dá PASS — até a emenda.
- Versão `0.1.0` igual em `Cargo.toml`, `pyproject.toml`, `jim.__version__` e `_core.VERSAO`.

## 7. Critérios de aceitação externos (v0.1.0)

- **A1** Um comando: em venv limpo com CPython ≥ 3.9 e Rust ≥ 1.85, `pip install .` instala `jim`
  sem dependência de runtime; `uvx --from . jim --version` imprime `jim 0.1.0`.
- **A2** `cargo build` e `cargo test --no-default-features` passam com Rust 1.85.
- **A3** `jim._core` expõe exatamente os nomes de `contract/jim_core.pyi`, com as mesmas
  assinaturas (`inspect.signature`); `python/jim/_core.pyi` é idêntico ao do contrato.
- **A4** `_core.IMPLEMENTADO is True`; a suíte diferencial roda sem pulo e aprova 100% dos casos,
  em CPython 3.9 e 3.13 (Linux).
- **A5** Todo item de `SEMANTICA.md` §7 tem ao menos um caso (DIFERENCIAL C6).
- **A6** Zero `PASS INDEVIDO` e zero `ACHADO CANDIDATO` (DIFERENCIAL C3, C5).
- **A7** `jim provar` nunca sai com 0; `jim verificar` sem `--reproduzir` nunca sai com 0.
- **A8** Ponta a ponta, num exemplo com um critério real: `provar` → 3; `verificar` → 3;
  `verificar --reproduzir` → 0; um byte trocado num arquivo do fecho → 1; um byte trocado no
  fecho → 1 ou 2, nunca 0; instrumento que lê relógio e muda a `saida` → reprodução 1.
- **A9** Esqueleto C0: toda função de `_core` levanta `NaoImplementado` (nenhuma devolve valor).
- **A10** Repositório público limpo: sem nome pessoal (só o handle XAKCN), sem caminho absoluto
  da máquina de desenvolvimento (exceto o exemplo documentado de `JIM_ORACULO` em
  `contract/DIFERENCIAL.md`), sem segredo ou token, sem arquivo de `.jim/` versionado.
- **A11** Testes só com biblioteca padrão (`unittest`); o CI não acessa o oráculo nem a rede,
  fora baixar crates e o maturin.

## 8. Emendas

| Id | Data | Mudança |
|---|---|---|
| C0 | 2026-11-27 | versão inicial do contrato |
| C0-1 | 2026-11-27 | Distribuição Python `jim-loops`, pois `jim` já identifica outro projeto. Módulo e comando continuam `jim`. |
| C0-2 | 2026-11-27 | `jim-fecho/1` ganha a chave `selo` (SEMANTICA §5.1, V4b em §5.3, item novo em §7 J). Motivo: A8 exige que um byte trocado no fecho nunca dê 0, mas `ambiente`, `duracao_s` e `contrato.caminho` não são recalculáveis; sem selo, trocá-los mantinha PASS. Interface (`.pyi`) sem mudança. |
| C0-3 | 2026-11-27 | V8 confere presença e quantidade de controles negativos contra o contrato (J-V8). Um controle fora do contrato invalidava a reprodução na v5-alpha e podia ser aceito pelas funções do núcleo se repetido em ambas as observações. |
| C0-4 | 2026-11-27 | §2.7 abre cada componente do inventário a partir do descritor do pai, sem seguir symlinks, e confere o descritor obtido. Sem essa abertura segura, o inventário de arquivos é recusado com J-C7. |
| C0-5 | 2026-11-27 | §5.2 exige que a montagem submeta os bytes do fecho ao mesmo leitor estrito de V2. Se excederem o limite de §2.1, a montagem devolve EntradaInvalida. |
| C0-6 | 2026-11-27 | §4.6 recusa execuções e controles com chaves ou tipos fora do formato gravado de §5.1, usando a mesma validação de formato do fecho. |
| C0-7 | 2026-11-27 | §4.2 exige caminhos válidos em `depende_de` mesmo sem a lista de arquivos resolvidos. Uma dependência inválida na projeção devolve EntradaInvalida R-2.12. |
| C0-8 | 2026-11-27 | §2.7 distingue caminho ausente (J-C4) de erro de acesso em qualquer componente (J-C7). |

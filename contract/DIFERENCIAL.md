# Testes diferenciais — Jim × v5-alpha (contrato C0)

O oráculo (Gauntlet Loops `v5-alpha`, commit `f761213`) é privado; o Jim é público e o CI do Jim
não o lê. Por isso o diferencial tem duas metades: um **gerador local**, que roda contra o oráculo
e grava casos em JSON versionados, e um **comparador no CI**, que só lê esses JSON e chama `jim._core`.

## 1. Peças (escritas pelo lado Python)

| Arquivo | Papel |
|---|---|
| `tests/diferencial/gerar.py` | gerador; só biblioteca padrão; casos sintéticos declarados no próprio código |
| `tests/diferencial/casos/{hash,cadeia,gate,derivacao,fecho}.json` | casos + saídas do oráculo (grupos H, K, G, D, J de SEMANTICA §7) |
| `tests/diferencial/casos/ORACULO.json` | identidade do oráculo e cobertura (§2, G9) |
| `tests/diferencial/test_diferencial.py` | comparador do CI (`unittest`) |
| `tests/diferencial/vigia.py` | registro de PASS (§4) |

Os arquivos de `casos/` só mudam por execução do gerador; ninguém os edita à mão.

## 2. Gerador

Uso local (o caminho é só exemplo):

```sh
JIM_ORACULO=/caminho/para/gauntlet-loops/base_4_5/project python -I -B tests/diferencial/gerar.py
```

- **G1** Sem `JIM_ORACULO`: sai com código 2. Nunca há caminho padrão.
- **G2** Identidade: `git -C "$JIM_ORACULO" rev-parse HEAD` começa por `f761213` e
  `git -C "$JIM_ORACULO" status --porcelain -- .` vem vazio; senão sai com 2.
- **G3** Somente leitura: `sys.dont_write_bytecode = True` e `PYTHONDONTWRITEBYTECODE=1` antes de
  importar; importa por `sys.path` (`$JIM_ORACULO` e `$JIM_ORACULO/tools`); arquivos temporários só
  em `tempfile.mkdtemp()`, apagados no fim. Ao terminar, repete o `status --porcelain` de G2; se
  mudou, falha e não grava nada.
- **G4** Casos sintéticos: não lê `trabalho/`, `releases/` nem outro dado do oráculo; só chama as
  funções de §5.
- **G5** Determinismo: mesmo oráculo + mesmo gerador + mesma versão do Python → mesmos bytes. Sem
  data, hora, caminho ou ordem de `set`. Escrita:
  `json.dumps(obj, ensure_ascii=True, sort_keys=True, indent=1) + "\n"` (ASCII puro; substituto
  isolado sai como `\udXXX` e volta igual no `json.loads` do teste).
- **G6** Valores que JSON não representa usam etiqueta (objeto de uma chave):
  `{"$bytes": "<base64>"}`, `{"$float": "nan"|"inf"|"-inf"}`, `{"$tuple": [...]}`. Inteiro grande
  é JSON nativo. Nenhum caso natural usa chave começando por `$`.
- **G7** Formato de cada caso:

```json
{"id": "G017", "grupo": "G", "funcao": "gate_43", "args": {"contrato": {}, "avaliacoes": {},
 "integracao": null, "hoje": "2026-01-01", "correntes": {}},
 "oraculo": {"chamada": "gate43.gate_43", "saida": {"tipo": "retorno", "valor": {}}},
 "esperado": {"tipo": "retorno", "valor": {}},
 "comparar": "exato", "divergencia": null, "limite": ["G: premissas null"], "regra": "§3.5"}
```

  `saida`/`esperado` de exceção: `{"tipo": "excecao", "classe": "ValueError"|"EntradaInvalida",
  "mensagem": "..."}`. Sem divergência, `esperado` sai mecanicamente de `oraculo` (ValueError →
  EntradaInvalida, mesma mensagem). Com divergência (`"divergencia": "J-G3"` etc., SEMANTICA §8),
  `esperado` é o que a SEMANTICA manda e `oraculo` fica como registro. Casos J (só Jim) têm
  `"oraculo": {"chamada": "gate43.gate_43", "gate": <saída do oráculo para o cenário>}` e
  `"fecho_sha256"` quando o cenário tem fecho.
- **G8** `comparar = "exato"` quando todo texto em `args` é ASCII imprimível; senão `"codigo"`.
- **G9** `ORACULO.json`: `{"tag": "v5-alpha", "commit": "f761213", "python": "X.Y.Z",
  "unicode": "<unicodedata.unidata_version>", "gerador_sha256": "<sha256 de gerar.py>",
  "limites": [<itens de SEMANTICA §7 cobertos>]}`. Nada além disso.
- **G10** Privacidade: antes de gravar, o gerador recusa (sai com 2) se a saída contiver o valor
  de `JIM_ORACULO`, `os.path.expanduser("~")`, `tempfile.gettempdir()`, `os.getcwd()`, o login
  (`getpass.getuser()`) ou o nome da máquina (`socket.gethostname()`). Não lê nem grava variáveis de ambiente além de `JIM_ORACULO`. Sem segredo,
  chave ou token em caso algum.

## 3. Correspondência Jim → oráculo

| `jim._core` | Oráculo | Conversão dos argumentos |
|---|---|---|
| `sha256_hex` | `hashlib.sha256(b).hexdigest()` | — |
| `json_estrito(b)` | `cadeia._carregar(b.decode("utf-8"))` | `UnicodeDecodeError` → `"R-5.16: UTF-8 inválido"` |
| `json_canonico(v)` | `json.dumps(v, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()` | — |
| `iguais` / `repr_py` | `release.iguais` / `repr` | — |
| `hash_fecho(a)` | `"sha256:" + release.hash_arvore(a)` | só entradas válidas em J-C/J-F; inválidas são casos só-Jim |
| `hash_linha` | `cadeia.hash_linha` | — |
| `avaliar_criterio_43`, `gate_43` | `gate43.avaliar_criterio_43`, `gate43.gate_43` | `hoje` data AAAA-MM-DD válida → `datetime.date.fromisoformat`; outro valor passa cru |
| `verificar_cadeia(b, a)` | `cadeia.verificar(arquivo_temporário_com_b, a)` | — |
| `eventos_cadeia` | `cadeia.eventos` | idem |
| `proxima_linha(b, e, a)` | `cadeia.acrescentar(tmp_com_b, e, a)` | `linha` = bytes acrescentados ao arquivo |
| `validar_contrato(c, arqs)` | `release.validar_contrato(m(c), "0.0", arqs)` | só regras em comum (compara código); J-K são só-Jim |
| `contrato_gate` | `release.contrato_gate(m(c), cand)` | — |
| `ultima_linha_json(t)` | `release.ultima_linha_json(n(t))` | `n`: `\r\n` e `\r` → `\n` (J-S1); J-S2 marcados |
| `avaliar_saida` | `release.avaliar_saida` | — |
| `controles_aprovados` | `release.controles_aprovados(x(ex), crit)` | — |
| `derivar_registros` | `release.registros(m(c), [x(e)…], exec, atuais, hc, hc_depois)` | — |
| `montar_fecho` | composição: `release.registros` + `contrato_gate` + `gate_43` + `hash_arvore` + `json.dumps` canônico | `esperado` = bytes do fecho |
| `verificar_fecho`, `comparar_reproducao` | — (só Jim; SEMANTICA §5) | `oraculo.gate` do cenário |

`m(c)` = `{"release": "0.0", "versao", "data_referencia", "arquivos", "criterios"}` de `c` (+ `premissas`
se houver). `x(ex)` = `{"criterio", "comando", "exit_code", "saida"}` e, por controle,
`{"alterado", "exit_code", "saida"}` + (`stderr_sha256` e `stderr_final` = `stderr.strip().splitlines()[-5:]`
se `stderr` não vazio). `saida` = `release.ultima_linha_json(n(stdout))`.

## 4. Comparador no CI e regra do PASS

- **C1** Para cada caso: decodifica etiquetas, chama `jim._core.<funcao>(**args)` e compara com
  `esperado`. `exato`: retorno com `iguais` (bytes comparados pela etiqueta `$bytes`); exceção
  `EntradaInvalida` com a mesma mensagem. `codigo`: mesma estrutura e mesmos `resultado`/`veredito`;
  cada `motivo` e cada mensagem comparados pelo código (texto até o primeiro `": "`).
- **C2** `NaoImplementado` nunca conta como aprovado. Com `jim._core.IMPLEMENTADO is False`, a suíte
  diferencial é pulada com aviso explícito; com True, qualquer `NaoImplementado` é falha.
- **C3** PASS onde `esperado` não é PASS: falha marcada `PASS INDEVIDO` (a mais grave da suíte).
- **C4** Vigia: em toda a suíte (não só no diferencial), `vigia.py` envolve `avaliar_criterio_43`,
  `gate_43`, `comparar_reproducao` e `verificar_fecho` e anota cada PASS com a chave
  `(funcao, sha256 dos args canônicos)` — ou `("fecho", fecho_sha256)` para as duas últimas. Testes
  de CLI em subprocesso que terminam em PASS registram a chave do fecho explicitamente.
- **C5** Regra: **todo PASS do Jim tem de bater com a v5-alpha num caso diferencial.** Uma chave
  anotada está coberta se existe caso com a mesma chave, `esperado` PASS e oráculo PASS (para os
  casos J: `oraculo.gate.veredito == "PASS"`). Chave sem cobertura imprime
  `ACHADO CANDIDATO: PASS sem caso diferencial: <funcao> <chave>` e faz a suíte falhar. A correção
  é acrescentar o cenário ao gerador e regenerar — nunca relaxar a vigia.
- **C6** Frescor e cobertura: `ORACULO.json.gerador_sha256 == sha256(gerar.py)`; `limites` contém
  todos os itens de SEMANTICA §7 (lista espelhada em `tests/diferencial/limites.py`).
- **C7** Privacidade no CI: os testes varrem `tests/diferencial/casos/` atrás de caminho absoluto
  de usuário ou temporário (padrões em `tests/diferencial/limites.py`) e de login/máquina do CI.

## 5. Fluxo

1. Mudou SEMANTICA ou `gerar.py` → rodar o gerador localmente com o oráculo.
2. Rodar de novo sem mudar nada → `git diff --exit-code tests/diferencial/casos` limpo (G5).
3. Commitar `gerar.py` e `casos/` juntos. O CI nunca regenera e nunca acessa a rede.

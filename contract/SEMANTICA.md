# SEMÂNTICA — `jim._core` 0.1.0 (contrato C0)

Este documento e `contract/jim_core.pyi` são a fonte da verdade do núcleo. "v5-alpha" é o
Gauntlet Loops na tag `v5-alpha` (commit `f761213`), repositório privado; nomes como
`gate43.gate_43` apontam a função equivalente de lá, mas este texto basta sozinho.
Onde a v5-alpha define o comportamento, o Jim é igual a ela; toda diferença tem código
`J-…` e está listada em §8. Mensagens entre aspas são exatas, com `{x}` = valor interpolado
por `str` e `{x!r}` = por `repr_py` (§2.4).

## §1 Domínio, termos e ordem

### §1.1 Domínio J-D (toda entrada anotada `Json`)

A conversão Python → núcleo percorre o argumento inteiro (inclusive partes que a regra
ignoraria), em profundidade e na ordem de inserção, e recusa no primeiro problema com
`EntradaInvalida`:

| Código | Condição | Mensagem |
|---|---|---|
| J-D1 | tipo não é exatamente `NoneType`, `bool`, `int`, `float`, `str`, `list` ou `dict` (subclasses, `tuple`, `set`, `bytes`, `datetime.date` etc. recusados) | `"J-D1: tipo fora do domínio JSON: {nome_do_tipo}"` |
| J-D1 | chave de `dict` não é `str` | `"J-D1: chave de objeto não é str: {nome_do_tipo}"` |
| J-D3 | `int` com \|v\| > 2^53 − 1 | `"R-5.16: inteiro fora de ±(2^53 - 1): {str(v)[:20]}…"` |
| J-D3 | `float` não finito ou \|v\| > 2^53 − 1 | `"R-5.16: número fora de ±(2^53 - 1) ou não finito: {repr(v)[:20]}"` |
| J-D4 | `str` (valor ou chave) com substituto UTF-16 isolado | `"R-5.16: texto com substituto UTF-16 isolado"` |
| J-D5 | mais de 64 níveis de `list`/`dict` (o próprio argumento conta 1) | `"R-5.16: mais de 64 níveis de aninhamento"` |

J-D vem antes de qualquer regra R-… da mesma chamada, para todos os argumentos, da esquerda
para a direita. Exceção: `ancora` de `verificar_cadeia`, `eventos_cadeia` e `proxima_linha` não
levanta erro J-D; fora de J-D ela conta como âncora malformada (§6.2). Parâmetro
`bytes`/`str`/`List[str]`/`Dict[str, str]` de outro tipo: `"J-D0: {nome_do_parametro} deve ser {tipo}"`.

### §1.2 Termos

- **ESPAÇO**: exatamente os caracteres de `str.isspace()` do Python: U+0009–U+000D,
  U+001C–U+001F, U+0020, U+0085, U+00A0, U+1680, U+2000–U+200A, U+2028, U+2029, U+202F,
  U+205F, U+3000. Atenção: U+001C–U+001F NÃO estão em `char::is_whitespace` do Rust.
- **strip(s)**: remove ESPAÇO das duas pontas. **id válido**: `str` com `strip(s) != ""`.
- **lista_ids(v)**: `list` não vazia cujos itens são todos id válido.
- **data AAAA-MM-DD**: `str` que casa inteira com `[0-9]{4}-[0-9]{2}-[0-9]{2}` (só dígitos ASCII,
  sem `\n` final) e é data gregoriana válida com ano 0001–9999.
- **ordem de código**: comparação lexicográfica por ponto de código (= bytes UTF-8); prefixo
  vem antes.
- **hex64**: `str` que casa com `[0-9a-f]{64}`.

### §1.3 Ordem

`dict` preserva a ordem de inserção na ida e na volta. Saídas seguem a ordem de chaves
escrita neste documento. `json_canonico` ignora a ordem (ordena).

## §2 Bytes, JSON e hash

### §2.1 `json_estrito(dados)` (= `cadeia._carregar` aplicado ao texto decodificado)

1. Decodifica UTF-8 estrito (sobrelongo, substituto codificado, truncado: erro)
   → `"R-5.16: UTF-8 inválido"`.
2. Profundidade, antes do parse: retira os literais de texto varrendo da esquerda: em cada `"`,
   tenta casar `"` + (qualquer caractere exceto `"` e `\`, ou `\` + qualquer caractere) * + `"`;
   se casar, o trecho sai; se não houver aspa de fecho, essa `"` fica e a varredura segue no
   caractere seguinte. No texto restante, `[`/`{` somam 1 e `]`/`}` subtraem 1; se o máximo
   passar de 64 → `"R-5.16: mais de 64 níveis de aninhamento"` (vale também para JSON inválido).
3. Parse com a gramática do `json.loads` do Python: espaço JSON só `' '`, `\t`, `\n`, `\r`;
   número `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][-+]?[0-9]+)?`; texto sem caractere < U+0020 cru;
   escapes `\" \\ \/ \b \f \n \r \t \uXXXX` (hex em qualquer caixa); `\uD8xx` seguido de
   `\uDCxx` vira um caractere; substituto sem par fica isolado (recusado no passo 5).
   Erros, no primeiro ponto da esquerda para a direita:
   - inteiro (sem fração e sem expoente) com mais de 16 dígitos ou \|v\| > 2^53 − 1
     → `"R-5.16: inteiro fora de ±(2^53 - 1): {lexema[:20]}…"` (`-0` vale 0);
   - número com fração/expoente: arredondamento correto para f64 (estouro = infinito);
     não finito ou \|v\| > 2^53 − 1 → `"R-5.16: número fora de ±(2^53 - 1) ou não finito: {lexema[:20]}"`
     (`9007199254740991.5` arredonda para 2^53 e é recusado; `1e-400` vale 0.0);
   - `NaN`, `Infinity`, `-Infinity` → `"R-5.16: {nome} não é número JSON finito"`;
   - qualquer outro erro (BOM, dado extra, vírgula sobrando, aspa aberta…) → `"R-5.16: JSON inválido"`.
4. Chave repetida em qualquer objeto, comparada já decodificada (`"a"` = `"a"`; par de
   substitutos escapado = caractere direto; substitutos isolados também comparam)
   → `"R-5.16: objeto com chave repetida (C4-4)"`. Só depois de um parse sem erro.
5. Substituto isolado em qualquer chave ou texto → `"R-5.16: texto com substituto UTF-16 isolado"`.

O resultado está sempre no domínio J-D. Números sem fração/expoente viram `int`; os demais, `float`.

### §2.2 `json_canonico(valor)`

Bit a bit igual a `json.dumps(valor, ensure_ascii=False, sort_keys=True,
separators=(",", ":")).encode("utf-8")`, sem `\n` final:

- `null`, `true`, `false`; `int` em decimal (`-` se negativo).
- `float` (finito, \|x\| ≤ 2^53 − 1): zero → `0.0` ou `-0.0`. Senão, `-` na frente se negativo e, sobre \|x\|, sejam `d` (n dígitos, sem
  zeros à direita) e `p` os dígitos mais curtos que fazem ida e volta e a posição do ponto
  (x = 0.d × 10^p), como o `repr` do Python. Se `p ≤ -4`: `d[0]` + (`.` + `d[1:]` se n > 1) +
  `e` + sinal (`-`/`+`) + \|p − 1\| com pelo menos 2 dígitos. Se `p ≤ 0` (e > -4): `0.` + `0`×(−p) + `d`.
  Se `p ≥ n`: `d` + `0`×(p − n) + `.0`. Senão `d[:p]` + `.` + `d[p:]`. (`p > 16` está fora do domínio.)
  Vetores: `1.0`, `1.5`, `0.1`, `0.0001`, `1e-05`, `2.5e-05`, `1.234e-07`, `5e-324`,
  `123456.789`, `1000000000000000.0`, `9007199254740991.0`, `0.30000000000000004`.
- texto: `"` + escape + `"`. Escapes: `"`→`\"`, `\`→`\\`, U+000A→`\n`, U+000D→`\r`,
  U+0009→`\t`, U+0008→`\b`, U+000C→`\f`, demais < U+0020 → `\u00xx` (hex minúsculo).
  Todo o resto sai cru em UTF-8, inclusive U+007F, U+2028, U+2029 e não-ASCII.
- lista: `[` + itens unidos por `,` + `]`. Objeto: `{` + `chave:valor` unidos por `,` + `}`,
  chaves em ordem de código.

Vetor: `{"z":0.0001,"y":1e-05,"x":-0.0,"é":" \x7f\x01","B":[1,2.0]}` →
`{"B":[1,2.0],"x":-0.0,"y":1e-05,"z":0.0001,"é":"<U+2028 cru><U+007F cru>\u0001"}`.

### §2.3 `iguais(a, b)`

`json_canonico(a) == json_canonico(b)`. Logo `True`, `1` e `1.0` diferem; `0.0` e `-0.0`
diferem; ordem de chave não importa; ordem de lista importa.

### §2.4 `repr_py(valor)` (texto dos motivos)

`None`, `True`, `False`; `int` decimal; `float` como §2.2; lista `[a, b]` (vírgula+espaço);
objeto `{'k': v, …}` na ordem de inserção; texto: aspas `'`, ou `"` se o texto tem `'` e
não tem `"`; escapes `\\`, a aspa escolhida, `\t`, `\n`, `\r`; demais < U+0020 e U+007F →
`\xNN` minúsculo; ASCII imprimível cru. Não-ASCII (J-R1): cru, exceto U+0080–U+00A0, U+00AD,
U+1680, U+2000–U+200F, U+2028–U+202F, U+205F–U+206F, U+3000, U+FEFF, U+E000–U+F8FF e
U+F0000–U+10FFFF, escapados como `\xNN` (≤ U+00FF), `\uNNNN` ou `\UNNNNNNNN`. O texto só é
normativo quando todos os valores interpolados são ASCII (DIFERENCIAL.md, modo `exato`).

### §2.5 `validar_caminho(caminho)`

Válido se: `str` não vazia; sem caractere < U+0020 nem U+007F; sem `\`; não começa com `/`;
partida em `/`, cada segmento é não vazio e diferente de `.` e `..` (logo sem `//`, `./` ou `/`
final). Senão `"J-C1: caminho relativo inválido: {caminho!r}"`. Os nomes `candidato` e
`contrato` são reservados (chaves de `correntes`): `"J-C2: caminho reservado: {caminho!r}"`.

### §2.6 `hash_fecho(arquivos)` — hash canônico do fecho

`arquivos` = {caminho: sha256 hex}. Vazio → `"J-F1: fecho vazio"`; caminho inválido → J-C1/J-C2;
valor que não é hex64 → `"J-F2: hash inválido em {caminho!r}"`. Resultado:
`"sha256:" + sha256_hex(UTF-8(concatenação, em ordem de código dos caminhos, de caminho + " " + hash + "\n"))`,
igual a `"sha256:" + release.hash_arvore(arquivos)`. Vetores (conferidos na v5-alpha):

- `{"b.py": "0"×64, "a/x.py": "f"×64}` → `sha256:9ba2313d796760aa2f69f6ef98ec9f55aab8d2b35a6cbc0dac9184a5f455f44c`
- `{"é.py": "1"×64, "Z.py": "2"×64, "a.py": "3"×64, "a b.py": "4"×64}` (ordem `Z.py`, `a b.py`,
  `a.py`, `é.py`) → `sha256:6e64fff17930dea8ed52dd5b1e2eaaceb182c5509555a5004c9a561cb429aa4f`

Internamente (§4.6) a mesma fórmula aceita dicionário vazio: `sha256("")` =
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

### §2.7 `inventariar(raiz, caminhos)`

`raiz`: diretório existente (resolvido uma vez; symlink na própria raiz é aceito). Cada caminho:
§2.5; repetido → `"J-C3: caminho repetido: {c!r}"`. Componentes intermediários têm de ser
diretórios e nenhum componente pode ser symlink → `"J-C5: symlink recusado: {c!r}"`;
ausente → `"J-C4: arquivo ausente: {c!r}"`; não regular → `"J-C6: não é arquivo regular: {c!r}"`;
erro de leitura → `"J-C7: arquivo ilegível: {c!r}"`. Devolve {caminho: sha256_hex(bytes)} na
ordem de `caminhos`. `raiz` que não é diretório → `"J-C8: raiz não é diretório: {raiz!r}"`.

C0-4: a partir do descritor da raiz resolvida, cada componente é aberto relativo ao
descritor do pai e sem seguir symlinks. O tipo do descritor aberto é conferido antes de
usá-lo. Symlink inserido entre a consulta e a abertura também é recusado com J-C5.
Plataformas sem essa abertura no núcleo recusam o inventário de arquivos com
`"J-C7: abertura segura indisponível: {c!r}"`.

## §3 Gate (semântica de `gate43`)

### §3.1 Constantes

CAMPOS = (`adequado`, `atual`, `falha_critica`, `conflito`); EXIGIDO = (True, True, False, False).
EXTERNOS = (`execucao`, `teste`, `medicao`, `fonte_externa`, `revisao_independente`);
INTERNOS = (`autocritica`, `critico_isolado`); COM_DATA = (`fonte_externa`, `medicao`);
VPD = `"vinculada_dependencias"`; SEVERIDADES = (`critica`, `alta`, `media`, `baixa`);
CHAVES_REGISTRO = CAMPOS + (`instrumento`, `dependencias`, `valido_ate`, `versao_contrato`).
`get(k)` de chave ausente e de `null` dão ambos None. Resultado `R(res, motivo)` =
`{"resultado": res, "motivo": motivo}`; PASS sempre com motivo None.

### §3.2 `avaliar(registro, hoje, aceitos, correntes, familias, exige_humano, versao)`

`avaliar_criterio_43` = `avaliar` sem `versao` (passo 1d e 2l não existem). Ordem fixa; a
primeira linha que se aplica decide. "EXC" = `EntradaInvalida`.

| # | Condição | Saída |
|---|---|---|
| E1 | `hoje` não é data AAAA-MM-DD | EXC `"R-2.14: hoje deve ser data AAAA-MM-DD, recebido {hoje!r}"` (J-G1) |
| E2 | `correntes` não é dict | EXC `"R-2.21: correntes deve ser dict, recebido {tipo}"` (`list`, `str`, `int`, `float`, `bool`, `NoneType`) |
| E3 | `exige_humano` informado e não é bool (None inclusive) | EXC `"R-2.11: exige_humano deve ser bool, recebido {v!r}"` |
| E4a | `registro` é dict e algum campo de CAMPOS (nessa ordem) presente não é bool | EXC `"R-2.11: {campo} deve ser bool, recebido {v!r}"` |
| E4b | `registro` é dict, `instrumento` é id válido e não está em EXTERNOS ∪ INTERNOS (sem strip: `" teste "` é desconhecido) | EXC `"R-2.13: instrumento desconhecido: {i!r}"` |
| E4c | `registro` é dict, `valido_ate` não é None nem VPD e não é `str` que casa o padrão | EXC `"R-2.14: valido_ate fora do formato AAAA-MM-DD: {v!r}"` |
| E4d | idem, casa o padrão mas não é data válida (ano 0000, 02-30…) | EXC `"R-2.14: valido_ate não é uma data válida: {v!r}"` |
| 1a | não lista_ids(`aceitos`) | UNKNOWN `"R-2.8: critério sem lista de instrumentos aceitos"` |
| 1b | `familias` não é None e não lista_ids | UNKNOWN `"R-2.19: lista de famílias de revisor aceitas malformada ou vazia"` |
| 1c | `exige_humano` e `familias != ["humano"]` | UNKNOWN `'R-2.19: critério de aprovação humana sem familias_revisor_aceitas igual a ["humano"]'` |
| 1d | `versao` não é id válido | UNKNOWN `"R-2.7: contrato sem versão"` |
| 2a | `registro` não é dict | UNKNOWN `"R-2.9: registro de evidência ausente"` |
| 2b | campo de CAMPOS ausente (primeiro na ordem) | UNKNOWN `"R-2.9: campo {campo} omitido"` |
| 2c | `dependencias` não é dict ou é vazio | UNKNOWN `"R-2.12: registro sem dependências identificadas"` |
| 2d | algum par (ordem de inserção) com chave ou valor que não é id válido | UNKNOWN `"R-2.12: dependência sem identificador: {chave!r}"` |
| 2e | `instrumento` não é id válido | UNKNOWN `"R-2.9: campo instrumento omitido"` |
| 2f | `instrumento` ∈ INTERNOS | FAIL `"R-2.13: instrumento interno não sustenta PASS: {i}"` |
| 2g | `instrumento` ∉ `aceitos` | FAIL `"R-2.13: instrumento fora da lista aceita: {i}"` |
| 2h | `instrumento == "revisao_independente"` e `familia_builder` ou `familia_revisor` não é id válido | UNKNOWN `"R-2.16: família do builder ou do revisor ausente"` |
| 2h' | idem e alguma das duas tem caractere não-ASCII | UNKNOWN `"J-G2: família fora do ASCII; comparação de caixa não é portável"` |
| 2h'' | idem e `lower(strip(fb)) == lower(strip(fr))` (minúscula ASCII) | FAIL `"R-2.16: revisor da mesma família do builder: {fr}"` |
| 2i | `valido_ate` None | UNKNOWN `"R-2.14: registro sem valido_ate"` |
| 2j | `valido_ate != VPD` e `hoje > valido_ate` (no próprio dia ainda vale) | FAIL `"R-2.14: evidência vencida em {valido_ate}"` |
| 2k | campo de CAMPOS (ordem) diferente de EXIGIDO | FAIL `"R-2.10: {campo} deve ser {True\|False}, registrado {True\|False}"` |
| 2l | `versao_contrato` None | UNKNOWN `"R-2.9: campo versao_contrato omitido"` |
| 2l' | `versao_contrato != versao` (só `str` igual é igual) | FAIL `"R-2.5: evidência da versão {vc!r} do contrato, vigente {versao!r}"` |
| 3 | `instrumento` ∈ COM_DATA e `valido_ate == VPD` | UNKNOWN `"R-2.23: {i} exige valido_ate datado, não vinculada_dependencias"` |
| 4a | chave de `dependencias`, em ordem de código, ausente de `correntes` ou com corrente que não é id válido | UNKNOWN `"R-2.21: dependência {k!r} sem identificador corrente"` |
| 4b | chave, em ordem de código, com valor ≠ corrente | FAIL `"R-2.21: dependência {k!r} mudou: registro {v!r}, corrente {c!r}"` |
| 5a | `familias` não é None e `familia_revisor` não é id válido | UNKNOWN `"R-2.19: registro sem familia_revisor"` |
| 5b | `familias` não é None e `familia_revisor ∉ familias` (igualdade exata, sem strip/caixa) | FAIL `"R-2.19: família de revisor não aceita: {fr}"` |
| 5c | `exige_humano` e `instrumento != "revisao_independente"` | UNKNOWN `"R-2.19: aprovação humana exige revisao_independente, registrado {i}"` |
| 5d | `exige_humano` e `familia_revisor != "humano"` | FAIL `"R-2.19: aprovação humana registrada pela família {fr!r}"` |
| 6 | nenhuma acima | PASS |

Todo o 4a roda antes do 4b. 4b compara só `str` (ambos já são id válido).

### §3.3 `avaliar_integracao(integracao, hoje, aceitos, correntes, candidato, versao)`

E1, E2, E4 (sobre `integracao`); então: I1 não lista_ids(`aceitos`) → UNKNOWN
`"R-2.22: contrato sem integracao.evidencias_aceitas"`; I2 `integracao` não é dict ou não tem
nenhuma chave de CHAVES_REGISTRO (ex.: `{"ok": true, "versao_entregue": …}` ou `true`) → UNKNOWN
`"R-2.22: integração não é registro de evidência no formato 2.2"`; I3 `candidato` não é id válido
→ UNKNOWN `"R-2.22: contrato sem candidato"`; I4 `dependencias` não é dict ou
`dependencias["candidato"]` não é id válido → UNKNOWN `"R-2.22: integração sem dependencias['candidato']"`;
I5 diferente de `candidato` → FAIL `"R-2.22: integração de outro candidato: {r!r}, contrato {c!r}"`;
I6 `avaliar(integracao, hoje, aceitos, correntes, None, False, versao)` — logo `candidato`
também precisa estar em `correntes` (4a/4b).

### §3.4 Contrato do gate (`_validar_contrato`; nunca levanta; só PASS ou UNKNOWN)

C1 não é dict ou `versao` não é id válido → `"R-2.7: contrato sem versão"`. C2 `candidato` não é
id válido → `"R-2.7: contrato sem candidato"`. C3 não lista_ids(`obrigatorios`) →
`"R-2.8: lista de critérios obrigatórios vazia ou malformada"`. C4 para cada obrigatório (ordem,
repetidos inclusive): `evidencias_aceitas` não é dict ou não lista_ids(seu valor) →
`"R-2.8: critério {c} sem instrumentos aceitos"`. C5 para cada: `severidade[c]` (None se
`severidade` não é dict) não é `str` ∈ SEVERIDADES → `"R-2.24: critério {c} sem severidade válida: {s!r}"`.
C6 `familias_revisor_aceitas` (ausente = `{}`; `null` = None) não é dict →
`"R-2.19: familias_revisor_aceitas deve ser dict por critério"`. C7 cada par (ordem) cujo valor
não é lista_ids → `"R-2.19: famílias de revisor aceitas malformadas em {c!r}"`. C8
`aprovacao_humana` (ausente = `[]`) não é list de ids válidos → `"R-2.19: aprovacao_humana malformada"`.
C9 para cada: fora de `obrigatorios` → `"R-2.19: critério de aprovação humana {c} fora dos obrigatórios"`;
`evidencias_aceitas[c]` não é exatamente `["revisao_independente"]` →
`'R-2.18: critério {c} de aprovação humana com evidencias_aceitas diferente de ["revisao_independente"]: {v!r}'`;
`familias[c]` não é exatamente `["humano"]` →
`'R-2.19: critério {c} de aprovação humana sem familias_revisor_aceitas igual a ["humano"]'`.
C10 `integracao` não é dict ou não lista_ids(`integracao.evidencias_aceitas`) →
`"R-2.22: contrato sem integracao.evidencias_aceitas"`. Senão PASS.

### §3.5 `gate_43(contrato, avaliacoes, integracao, hoje, correntes)`

1. E1, E2. `avaliacoes` não é dict → EXC `"R-2.9: avaliacoes deve ser dict ou pares (criterio, registro)"`
   (J-G3: pares não são aceitos).
2. `rc` = §3.4. `c` = contrato se dict, senão `{}`. `obrig` = itens id válido de
   `c.obrigatorios` se list, senão `[]`. `aceitas`, `sev`, `fam` = o campo se dict, senão `{}`
   (aqui `null` também vira `{}`); `hum` = o campo se list, senão `[]`; `versao` = `c.get("versao")`.
   `premissas` = `c.get("premissas", {})`; se não é dict: `rc` = UNKNOWN
   `"R-2.17: premissas deve ser dict por critério"` (substitui o motivo anterior) e `premissas = {}`.
3. Para cada `k` em `obrig` (ordem; repetido reavalia e mantém a primeira posição):
   `r = avaliar(avaliacoes.get(k), hoje, aceitas.get(k), correntes, fam.get(k), k in hum, versao)`.
   Exceção de E4 aqui propaga (a do primeiro obrigatório na ordem). Registros fora de `obrig`
   não passam por E4. Se `r` é PASS e `k in premissas`: `ids = premissas[k]`; se não é list ou
   algum item não é id válido ou é `"candidato"` → UNKNOWN `"R-2.17: identificadores de premissas inválidos"`;
   senão o primeiro `p` fora de `registro["dependencias"]` → UNKNOWN
   `"R-2.17: premissa {p!r} sem verificação vinculada"` (`[]` mantém PASS).
   `r["severidade"]` = `sev.get(k)` se `str` ∈ SEVERIDADES, senão None. Saída por critério:
   `{"resultado", "motivo", "severidade"}`.
4. Integração: §3.3 com `aceitos` = `c.integracao.evidencias_aceitas` (None se `c.integracao`
   não é dict), `candidato` = `c.get("candidato")`. E4 da integração propaga depois de todos os critérios.
5. Veredito: `rc` ≠ PASS → UNKNOWN; senão, critérios não vazios e todas as partes (critérios +
   integração) PASS → PASS; senão alguma FAIL → FAIL; senão UNKNOWN.
6. `ordem_falhas` = critérios não PASS ordenados por (posto, nome em ordem de código); posto =
   índice em SEVERIDADES, ou −1 se severidade None (vem primeiro).
7. Saída: `{"veredito", "contrato": rc, "criterios", "integracao", "ordem_falhas"}`.

### §3.6 Garantias contra PASS indevido (obrigatórias na implementação)

- PASS só nasce na linha 6 de §3.2 e no passo 5 de §3.5; não existe valor padrão PASS,
  `unwrap_or(PASS)`, `_ => PASS` nem PASS em caminho de erro.
- veredito PASS ⇒ `rc` PASS ∧ ≥ 1 critério ∧ todos PASS ∧ integração PASS.
- Tipo inesperado em qualquer campo: a regra que o lê trata como "não é id válido"/"não é dict"
  (UNKNOWN) ou levanta (E…); nunca como verdadeiro. Pânico Rust é capturado e vira
  `EntradaInvalida("J-0: falha interna: …")`.
- `gate_43` nunca devolve UNPROVEN; UNPROVEN só existe em §5.

## §4 Derivação de registros (semântica de `tools/release.py`)

### §4.1 `validar_contrato(c, arquivos)` — contrato `jim-contrato/1`

`arquivos` = caminhos resolvidos (§2.5, sem repetição). Ordem e mensagens:
K1 não é dict → `"J-K1: contrato deve ser objeto"`. K2 `esquema != "jim-contrato/1"` →
`"J-K2: esquema deve ser jim-contrato/1"`. K3 chave fora de {`esquema`, `versao`,
`data_referencia`, `arquivos`, `criterios`, `premissas`} → `"J-K5: chave desconhecida: {k!r}"`.
K4 `versao` não é id válido → `"R-2.1/R-2.5: identidade do contrato inválida"`. K5
`data_referencia` não é data AAAA-MM-DD → `"R-2.14: data_referencia fora do formato AAAA-MM-DD"`.
K6 `arquivos` não é list não vazia de `str` não vazias → `"J-K3: arquivos deve ser lista não vazia de padrões"`.
K7 `criterios` não é dict não vazio, ou tem a chave `integracao` → `"R-2.8: critérios ausentes ou reservados"`.
Para cada critério (ordem): nome não é id válido ou valor não é dict → `"R-2.8: critério malformado"`;
chave fora de {`comando`, `aceitos`, `severidade`, `reprova`, `esperado`, `regra`, `depende_de`,
`timeout_s`, `controles_negativos`} → J-K5; `severidade` ∉ SEVERIDADES → `"R-2.24: severidade obrigatória: {nome}"`;
`comando` não é list não vazia de `str` não vazias → `"R-2.1: comando inválido: {nome}"`;
`comando[0]` ∉ `arquivos` ou não termina em `.py` → `"R-3.13: instrumento Python fora da cópia"`;
`reprova` não é id válido → `"R-2.1: condição de reprovação ausente"`; `depende_de` (padrão:
`arquivos`) não é list não vazia contida em `arquivos` → `"R-2.12: dependências fora do candidato"`;
`timeout_s` (padrão 900) não é `int` > 0 → `"R-5.3: timeout inválido"`; `aceitos` não é list não
vazia → `"R-2.8: instrumentos aceitos ausentes"`; sem `esperado` e `regra != "todos_aprovados"`
→ `"R-2.1: regra inválida"` (J-K4: `sem_sobreviventes*` fora da 0.1); `controles_negativos`, se
presente, não é list não vazia de dicts não vazios cujas chaves estão em `depende_de`, não
terminam em `.py` e cujos valores são `str` ou None →
`"R-3.4: controles negativos exigem alterações de artefatos do critério: {nome}"`.

### §4.2 `contrato_gate(c, candidato)`

`{"versao": c.versao, "candidato": candidato, "obrigatorios": [nomes na ordem],
"evidencias_aceitas": {nome: aceitos}, "severidade": {nome: severidade},
"integracao": {"evidencias_aceitas": ["execucao"]}}` e, se `c` tem `premissas`,
`"premissas": c.premissas` (ausência preservada).

### §4.3 `ultima_linha_json(texto)`

1. J-S1: troca `\r\n` e depois `\r` por `\n` (emula a leitura em modo texto da v5-alpha; o
   stream gravado não muda). 2. `t = strip(texto)`. 3. Quebra `t` em linhas como
   `str.splitlines()`: separadores `\n`, `\r\n`, `\r`, U+000B, U+000C, U+001C, U+001D, U+001E,
   U+0085, U+2028, U+2029; separador final não gera linha vazia. 4. Da última para a primeira,
   a PRIMEIRA linha com `strip(linha)` começando por `{` decide: `json_estrito(linha)` (a linha
   sem strip; só espaço JSON é tolerado) ou None se der erro — não volta para linhas anteriores.
   5. Nenhuma linha assim → None. J-S2: o parse é o de §2.1 (a v5-alpha aceitava inteiro grande,
   `1e400` e profundidade > 64 aqui); recusado vira None.

### §4.4 `avaliar_saida(saida, crit)`

`saida` não é dict → False. Se `crit` tem `esperado` → `iguais(saida, esperado)`. Se
`regra == "todos_aprovados"` → `casos` int (não bool) > 0 ∧ `aprovados` int == `casos` ∧
`falhas == []` ∧ `erros_enumeracao` (ausente = 0) é int == 0. Outra regra → EXC
`"R-2.1: critério sem esperado/regra reconhecida"`.

### §4.5 `controles_aprovados(ex, crit)`

True se e só se: `crit.controles_negativos` não vazio; `ex.controles_negativos` é list do mesmo
tamanho; e cada controle gravado tem `alterado is True`, `exit_code` int ∈ {0, 1},
`stderr == ""` (J-S3: equivale ao "sem stderr_sha256/stderr_final" da v5-alpha) e rejeição
observada em `saida`:
- `saida` não é dict, é vazio ou tem `erro` → False; `erros` (ausente = 0) tem de ser int 0 ou
  list/dict vazio; `estado == "UNKNOWN"` → False.
- com `esperado`: `formato(saida, esperado)`; `casos` (ausente = 1) int > 0;
  `erros_enumeracao` (ausente = 0) int == 0; e `not avaliar_saida(saida, crit)`.
- com `todos_aprovados`: `casos` int > 0, `aprovados` int com 0 ≤ aprovados < casos, `falhas`
  list com `len == casos − aprovados`, cada falha (se dict, seu `caso`, senão `bateria`) é id
  válido, `erros_enumeracao` (ausente = 0) int == 0.
- `formato(s, e)`: tipos exatamente iguais (bool ≠ int ≠ float); dict: mesmo conjunto de chaves e
  `formato` em cada; list não vazia: cada item de `s` casa com algum modelo de `e`; demais: True.

### §4.6 `derivar_registros(c, execucoes, executados, atuais, hc, hc_depois)`

Pré: `execucoes` no formato gravado (§5.1), uma por critério na ordem do contrato (senão EXC
`"R-2.1: execuções diferem dos critérios"`), com `comando == ["python"] + comando` e `exit_code`
int (senão EXC `"R-2.1: instrumento gravado diverge"`), e cada stream conferido (J-N7: `stdout_sha256 == sha256(stdout)`, idem stderr, `iguais(saida,
ultima_linha_json(stdout))`, também nos controles; senão EXC `"J-N7: stream gravado não confere: {criterio}"`).
Depois dessas conferências, a derivação valida as chaves e os tipos da execução e dos controles
gravados com as regras de V4 (C0-6). Se o formato divergir, EXC
`"R-2.1: execução gravada malformada: {criterio}"`.
Com `T(d)` = fórmula de §2.6 (aceita vazio):
- `candidato = "sha256:" + T(executados)`.
- `correntes` = {p: `"sha256:" + h`} de `atuais` (ordem de `atuais`) + `candidato: "sha256:" + T(atuais)`
  + `contrato: "sha256:" + hc_depois`.
- para cada critério: `deps` = {p: `"sha256:" + executados[p]`} para p em `depende_de` (padrão:
  chaves de `executados`) + `candidato` + `contrato: "sha256:" + hc`; registro =
  `{"adequado": exit_code == 0 ∧ avaliar_saida(saida, crit), "atual": todo deps[p] == correntes.get(p),
  "falha_critica": exit_code != 0 ∨ saida is None, "conflito": false, "instrumento": "execucao",
  "dependencias": deps, "versao_contrato": c.versao, "valido_ate": "vinculada_dependencias"}`;
  se (severidade `critica` ou há `controles_negativos`) e `adequado` e não §4.5 → remove a chave
  `adequado` (o gate dá UNKNOWN R-2.9).
- `confere = (executados == atuais) ∧ (hc == hc_depois)`; `integracao` = mesmo formato com
  `adequado = atual = confere`, `falha_critica = conflito = false`,
  `dependencias = {"candidato": candidato, "contrato": "sha256:" + hc}`.

## §5 Fecho e veredito do Jim

### §5.1 Documento `jim-fecho/1`

Chaves exatas: `esquema` (`"jim-fecho/1"`), `jim` (`"0.1.0"`), `contrato` (`{"caminho",
"sha256", "sha256_depois", "versao"}`), `data_referencia`, `candidato`, `arquivos`,
`arquivos_depois` (chaves ⊆ `arquivos`; ausente = apagado), `execucoes`, `evidencias`,
`integracao`, `correntes`, `contrato_gate`, `gate`, `veredito`, `ambiente` ({str: str},
informativo), `selo` (C0-2: `sha256_hex(json_canonico(fecho sem a chave selo))`; liga os campos
informativos — `ambiente`, `duracao_s`, `contrato.caminho` — ao documento, para que um byte
trocado nunca dê PASS; adulteração deliberada continua exigindo âncora, V1). Execução gravada = `Execucao` (pyi) + `saida`, `stdout_sha256`,
`stderr_sha256` (SHA-256 do UTF-8 do stream); controle gravado = `Controle` + os mesmos três.
Em disco: `json_canonico(fecho) + b"\n"`; `fecho_sha256` = SHA-256 desses bytes.

### §5.2 `montar_fecho`

M1 `c = json_estrito(contrato_bytes)`, `hc = sha256_hex(contrato_bytes)`. M2 §4.1 com as chaves
de `arquivos`; `hash_fecho(arquivos)`; `arquivos_depois` válido (§2.5, hex64, chaves ⊆);
`contrato_sha256_depois` hex64; `ambiente` {str: str}. M3 `execucoes` como em §4.6 (sem
`saida`/hashes; `controles_negativos` presente se e só se o critério tem controles, mesmo
tamanho; sem chave extra). M4 grava `saida` e hashes. M5 §4.6, §4.2 e §3.5 com
`hoje = c.data_referencia`. M6 `veredito` = UNPROVEN se o gate deu PASS, senão o veredito do
gate; grava o `selo` (C0-2). M7 valida os bytes finais com `json_estrito` (§2.1), o mesmo
leitor de V2 (C0-5). Qualquer violação: EXC. O fecho NUNCA tem veredito PASS: a execução foi do agente.

### §5.3 `verificar_fecho` (para no primeiro passo que falha; `erros = [motivo]`)

| # | Verificação | Falha → veredito, motivo |
|---|---|---|
| V0 | G2; `atuais` {caminho válido: hex64}; `ancora` None ou hex64; `reproducao` None ou formato do pyi | EXC (vêm do Jim) |
| V1 | `ancora` dada e ≠ `fecho_sha256` | FAIL `"J-V1: fecho não confere com a âncora externa"` |
| V2 | `json_estrito(fecho_bytes)` | UNKNOWN `"J-V2: fecho ilegível: {erro}"` |
| V3 | bytes == `json_canonico(f) + b"\n"` | UNKNOWN `"J-V3: fecho fora da forma canônica"` |
| V4 | chaves e tipos de §5.1; `veredito` ∈ {PASS, FAIL, UNKNOWN, UNPROVEN} (PASS gravado cai em V9) | UNKNOWN `"J-V4: fecho malformado: {campo}"` |
| V4b | `selo == sha256_hex(json_canonico(f sem selo))` (C0-2) | FAIL `"J-V4b: selo do fecho não confere"` |
| V5 | `json_estrito(contrato_bytes)`; depois `sha256 == f.contrato.sha256` | UNKNOWN `"J-V5: contrato ilegível: {erro}"`; FAIL `"R-2.5: contrato diverge do registrado no fecho"` |
| V6 | §4.1 com as chaves de `f.arquivos`; `versao`/`data_referencia` iguais aos do fecho | UNKNOWN `"J-V6: contrato inválido: {erro}"`; FAIL `"R-2.5: identidade do contrato diverge"` |
| V7 | `hash_fecho(f.arquivos) == f.candidato` | FAIL `"R-3.13: hash da árvore diverge do candidato"` |
| V8 | execuções/comandos (§4.6) e streams (J-N7); a presença e o número de controles correspondem ao contrato (§5.2 M3) | FAIL com a mensagem de §4.6 ou `"J-V8: controles negativos diferem do contrato: {criterio}"` |
| V9 | recalcula §4.6, §4.2, §3.5 e o veredito do fecho; `iguais` com o gravado | FAIL `"R-2.21: evidências/integração não se reconstroem"` / `"J-V9: veredito gravado diverge do recalculado"` |
| V10 | chaves de `atuais` (globs do contrato resolvidos agora) == chaves de `f.arquivos`; depois valores | FAIL `"R-3.13: arquivos do contrato diferem do fecho"` / `"R-2.21: árvore atual diverge do fecho: {primeiro caminho em ordem de código!r}"` |
| V11 | gate recalculado ≠ PASS | o veredito do gate; motivo = primeiro não nulo entre contrato, `criterios[ordem_falhas[0]]`, integração |
| V12 | `reproducao` None | UNPROVEN `"J-P1: PASS do gate sem reprodução externa"` |
| V13 | §5.5 | o resultado de §5.5 |

### §5.4 Veredito final

| Situação | Veredito |
|---|---|
| fecho ou contrato ilegível/malformado | UNKNOWN |
| âncora, contrato, árvore, stream ou registros divergentes | FAIL |
| gate recalculado FAIL / UNKNOWN | FAIL / UNKNOWN |
| gate PASS, sem reprodução ou com reprodução de outra cópia/incompleta | UNPROVEN |
| gate PASS, reprodução divergente ou cujo gate não dá PASS | FAIL |
| gate PASS e reprodução P7 | **PASS** (único caminho) |

### §5.5 `comparar_reproducao(f, contrato_bytes, rep)` — reprodução fora do agente

P0 formato (pyi `Reproducao`) e streams `str`; senão EXC. P2 `inventario_antes != f.arquivos` →
UNPROVEN `"J-P2: cópia da reprodução não é o fecho"`. P3 execuções não são uma por critério, na
ordem, com o mesmo `comando` e o mesmo número de controles → UNPROVEN
`"J-P3: reprodução incompleta ou fora de ordem"`. P4 por critério: `exit_code` ≠ gravado ou
`not iguais(ultima_linha_json(stdout), saida gravada)` → FAIL
`"J-P4: reprodução diverge do gravado: {criterio}"`. P5 por controle (i a partir de 1):
`alterado`, `exit_code`, saída derivada ou `stderr` diferentes do gravado → FAIL
`"J-P5: controle negativo diverge do gravado: {criterio}#{i}"`. P6 §4.6 com
(execuções da reprodução em forma gravada, `inventario_antes`, `inventario_depois`, hc, hc) e
§3.5 sobre a reprodução: veredito ≠ PASS →
FAIL `"J-P6: gate da reprodução não dá PASS: {motivo como V11}"`. P7 PASS. Stdout byte a byte
não é exigido igual (tempo, ordem); exigido é o stream inteiro presente (tools-N7).

## §6 Cadeia de hashes (semântica de `cadeia`)

### §6.1 Formato

Uma linha por evento, UTF-8, terminada em `\n`; objeto com exatamente `seq`, `anterior`, `evento`;
`seq` começa em 1 e cresce de 1; `anterior` = SHA-256 hex dos bytes exatos da linha anterior sem
`\n` (na primeira, GENESE); hash da linha = SHA-256 dos bytes lidos, sem reserializar. Vetor:
`proxima_linha(b"", {"texto": "ação", "n": 1.5, "l": [None, True]})` →
`{"anterior":"<GENESE>","evento":{"l":[null,true],"n":1.5,"texto":"ação"},"seq":1}\n`,
âncora `{"n": 1, "cabeca": "6d114df273aba50d44a82f71dbbcb971df099e81feadbc3aa88ab985ebf522e9"}`;
em seguida `{"i": 2}` → âncora `{"n": 2, "cabeca": "b03936983a4442a04a2fdd32696f89db38a13d7826133981b01a32f8a3abfd53"}`.

### §6.2 `verificar_cadeia(dados, ancora)` — nunca levanta por conteúdo

Início: `{"integra": False, "ancorada": None, "n": 0, "cabeca": GENESE, "erro": None}`; em erro,
devolve isso só com `erro` (n e cabeca NÃO refletem linhas boas anteriores).
1. `b""` → cadeia vazia. Não termina em `\n` → `"R-5.16: arquivo não termina em \n"` (barra e n
   literais). 2. Parte `dados[:-1]` em `\n` e decodifica TODAS as linhas antes de qualquer parse;
   falha → `"R-5.16: UTF-8 inválido"`. 3. Linha i (a partir de 1): §2.1 passos 2–5 →
   `"R-5.16: linha {i}: {mensagem sem o prefixo 'R-5.16: '}"`; não é objeto com exatamente as três
   chaves → `"R-5.16: linha {i}: chaves devem ser exatamente ['anterior', 'evento', 'seq']"`; `seq`
   não é int (bool não vale) ou ≠ i → `"R-5.16: linha {i}: seq {seq!r} fora de ordem"`;
   `anterior` ≠ hash da anterior → `"R-5.16: linha {i}: anterior não confere com o hash da linha {i-1}"`;
   `evento` não é objeto → `"R-5.16: linha {i}: evento deve ser objeto"`. (`\r` antes do `\n` é
   espaço JSON e entra no hash.)
4. Íntegra: `integra` True, `n`, `cabeca`. Só então a âncora: None → `ancorada` None. Não é dict
   com `n` int (não bool) e `cabeca` str, ou está fora de J-D → `ancorada` False,
   `"R-5.16: âncora deve ser {'n': int, 'cabeca': str}"`. Senão `ancorada = (n == ancora.n ∧
   cabeca == ancora.cabeca)`; False → `"R-5.16: âncora não confere: linhas removidas ou acrescentadas fora da cadeia"`.
   Cadeia quebrada com âncora dada: `ancorada` fica None. (J-D7: âncora fora de J-D, como `n`
   inteiro grande, é malformada; na v5-alpha seria "não confere"; `ancorada` False nos dois.)

### §6.3 `eventos_cadeia(dados, ancora)`

Se `integra` e `ancorada is True`: lista dos `evento`; senão EXC
`"R-5.16: cadeia não utilizável: {erro ou 'sem âncora'}"`.

### §6.4 `proxima_linha(dados, evento, ancora)`

J-D do `evento` primeiro (J-D6: na v5-alpha a cadeia era conferida antes). Depois: `evento` não é
dict → `"R-5.16: evento deve ser dict"`; §6.2 não íntegra →
`"R-5.16: cadeia quebrada, nada foi acrescentado: {erro}"`; `ancora` dada e `ancorada` não True →
`"R-5.16: cadeia não confere com a âncora: {erro}"`; texto = `json_canonico({"seq": n+1,
"anterior": cabeca, "evento": evento})`; texto passa por §2.1 passos 2–5 (ex.: profundidade 64 do
evento dá 65 na linha → recusa). Devolve `linha = texto + b"\n"` e
`ancora = {"n": n+1, "cabeca": sha256_hex(texto)}`.

## §7 Casos-limite obrigatórios (viram casos em tests/diferencial)

Cada item abaixo vira ao menos um caso em `tests/diferencial/casos/*.json`; o `id` do caso começa
pela letra do grupo (H, K, G, D, J) e o campo `limite` cita o item (ex.: `"G: premissas null"`).

- **H (hash/JSON)**: vetores de §2.2, §2.6 e §6.1; ordem de chave com maiúscula, `é` e
  `e`+U+0301; `True`/`1`/`1.0` e `0.0`/`-0.0` em `iguais`; 2^53−1 aceito e 2^53 recusado (int e
  float); `9007199254740991.5`; 17 dígitos; `1e400`; `1e-400`; `NaN`/`Infinity`/`-Infinity`;
  número inválido antes de chave repetida; `"a"` × `"a"`; par escapado × caractere direto;
  `"\ud800"` isolado, inclusive como chave repetida; profundidade 64 × 65; colchetes dentro de
  texto e em texto sem fecho; BOM; controle cru em texto; dado extra; `repr_py` de texto com `'`,
  `"`, ambos, `\t`, `\x01`, `\x7f`.
- **K (cadeia)**: íntegra e ancorada; sem âncora; `b""`; sem `\n` final; UTF-8 inválido
  na linha 3 com JSON ruim na linha 1; chave a mais/a menos; `seq` `true`/`1.0`/`"1"`; `anterior`
  errado; `evento` lista; cadeia quebrada com âncora (None); âncora malformada (`n` bool,
  `cabeca` int, lista); âncora divergente em n e em cabeça; remoção da última linha; CRLF;
  `proxima_linha` igual byte a byte a `acrescentar`; recusa em cadeia quebrada; eventos sem âncora.
- **G (gate)**: um caso por linha de §3.2 (E1–E4d, 1a–6), §3.3 (I1–I6) e §3.4 (C1–C10);
  precedência E1 > E2 > E4 do 1º obrigatório > E4 do 2º > E4 da integração; registro fora dos
  obrigatórios com `adequado: 1` não levanta; `premissas: null` substituindo o motivo de `rc`;
  premissa `"candidato"`; `premissas: {"a": []}`; `ordem_falhas` com severidade inválida e empate;
  obrigatórios repetidos e com id inválido; integração 4.2 e `true`; `candidato` ausente de
  `correntes`; família `"x\x1c"` × `"X"` (FAIL); família não-ASCII (J-G2); `"Humano"`;
  `valido_ate` `"2026-02-30"`, `"2026-1-01"`, `"0000-01-01"`, véspera/dia/dia seguinte;
  `" teste "`; instrumento `""`; `versao_contrato` ausente/`null`/`1`/lista; `avaliacoes` em pares
  (J-G3); `exige_humano=None`; duas dependências ausentes (relata a menor em ordem de código);
  ordem de inserção no R-2.12; `correntes` lista; `hoje` malformado.
- **D (derivação)**: PASS com `esperado`; `todos_aprovados`; `critica` sem controles
  (UNKNOWN R-2.9); controle com stderr; controle com `erro`; `formato` bool × int; última linha
  JSON com lixo depois, CRLF, U+001C, `"\xa0{"`, última `{` inválida sem recuo; exit ≠ 0; saída
  None; arquivo mudado e apagado na execução; contrato mudado; `regra` `sem_sobreviventes` (J-K4).
- **J (fecho, só Jim)**: fecho com gate PASS → UNPROVEN; verificar sem reprodução →
  UNPROVEN; reprodução igual → PASS; exit divergente → FAIL; critério faltando → UNPROVEN;
  cópia diferente → UNPROVEN; bytes não canônicos → UNKNOWN; stdout adulterado → FAIL; `saida`
  adulterada → FAIL; âncora → FAIL; árvore atual mudada → FAIL; contrato trocado → FAIL; fecho
  com `"veredito": "PASS"` gravado → FAIL (J-V9); controle reproduzido com stderr → FAIL;
  controle reproduzido com saída ou exit diferentes apesar de rejeição válida → FAIL (J-P5);
  campo informativo (`ambiente`) trocado sem refazer o selo → FAIL (J-V4b, C0-2);
  controles fora do contrato → FAIL (J-V8, C0-3).

## §8 Divergências intencionais em relação à v5-alpha

Todas tornam o Jim mais restritivo ou só mudam o formato; nenhuma cria PASS onde a v5-alpha não dá.

| Código | v5-alpha | Jim | Motivo |
|---|---|---|---|
| J-D1–J-D5 | aceita tuple/set, inteiro e float grandes no gate | EXC na fronteira | entrada nasce de JSON; um domínio só |
| J-G1 | `hoje: datetime.date` | `"AAAA-MM-DD"` | JSON; mesma regra R-2.14 |
| J-G2 | `lower()` Unicode do Python na família | não-ASCII → UNKNOWN | caixa Unicode muda por versão; FAIL→PASS seria possível |
| J-G3 | `avaliacoes` em pares | só dict | JSON |
| J-C1–J-C7 | caminho com `./`, `//`, symlink interno | recusa | um caminho, um hash |
| J-C2 | arquivo `candidato`/`contrato` sobrescreve `correntes` | recusa | colisão silenciosa |
| J-F1 | `hash_arvore({})` válido | recusa | fecho vazio não prova nada |
| J-K1–J-K5 | contrato de release (`release`, `fontes`…), chaves extras ignoradas, regras de mutação | `jim-contrato/1`, chave extra recusada, só `esperado`/`todos_aprovados` | enxuto; erro de digitação não some |
| J-D6 | cadeia conferida antes do domínio do evento | domínio antes | fronteira única |
| J-D7 | âncora com inteiro grande "não confere" | "âncora deve ser…" | ambas `ancorada` False |
| J-S1 | stdout lido em modo texto | bytes crus; normaliza só para achar a última linha | streams exatos (N7) |
| J-S2 | `release.json_estrito` frouxo na última linha | §2.1 | um parser; recusa vira None (FAIL) |
| J-S3 | controle sem `stderr_sha256`/`stderr_final` | `stderr == ""` | streams inteiros (N7) |
| J-N7 | evidência guarda só hashes | exige streams inteiros e confere hash e saída | CONTRATO.md §5 |
| J-R1 | `repr` com tabela Unicode do Python | tabela fixa para não-ASCII | texto não-ASCII não é normativo |
| J-V8 | verificador compara os controles declarados com os gravados durante a reexecução | o fecho confere presença e quantidade antes da reprodução | validar o fecho contra o contrato antes de usar a evidência gravada |
| J-P | PASS do gate encerra | PASS só com reprodução (§5.4) | ideia central do Jim |

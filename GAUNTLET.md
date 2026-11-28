# Gauntlet no Jim

O loop obriga o agente a **provar**. O agente registra a evidência com `jim provar` e nunca declara PASS. O agente reporta no máximo **UNPROVEN**, mesmo que observe saída 0. **PASS** só vale quando quem verifica reproduz fora do agente com seu contrato de confiança e obtém saída 0.

## Gancho

Ao ver a palavra `gauntlet` junto do link do repositório Jim ou desta página, leia esta página, detecte o ambiente na ordem abaixo e responda na primeira linha no formato `gauntlet armado: modo X (descrição curta); teto Y`. Use a linha exata do modo detectado.

- `gauntlet armado: modo A (terminal com rede); teto UNPROVEN até reprodução fora do agente`
- `gauntlet armado: modo B (só web); teto UNPROVEN`
- `gauntlet armado: modo C (terminal sem rede); teto UNPROVEN`

## Detecte o ambiente

Se houver ferramenta de terminal, faça uma única sonda de rede.

```sh
curl -sS -m 5 -I https://github.com
```

### Modo A — terminal com rede

A sonda recebeu resposta HTTP. Rode o Jim na raiz de um clone do repositório.

```sh
uvx --from git+https://github.com/XAKCN/Jim jim provar --raiz exemplos/minimo
```

Se você só tem o link, pode salvar os três arquivos raw abaixo em `exemplos/minimo/` e executar a partir do diretório que contém `exemplos/`. O `uvx` não deixa `jim` no PATH. Repita a chamada com `uvx` para cada comando ou use um venv.

```sh
python3 -m venv .venv && .venv/bin/pip install git+https://github.com/XAKCN/Jim
.venv/bin/jim provar --raiz exemplos/minimo
```

O venv evita a restrição PEP 668 (`externally-managed-environment`) do pip do sistema. Com rede, o maturin baixa Rust automaticamente se não houver `rustc >= 1.85`. A primeira instalação leva cerca de 35 s.

Entregue o fecho de `.jim/`, a versão dos arquivos e o comando. `jim provar` lê por padrão o `RAIZ/jim.json` do candidato. Quem verifica fornece em `--contrato` uma cópia de confiança independente do agente. Um fecho feito com critérios enfraquecidos pelo agente só passa pela conferência do contrato se a cópia de confiança tiver os mesmos bytes. Para o exemplo mínimo, use o `exemplos/minimo/jim.json` da cópia confiável do repositório.

Em outro ambiente, quem verifica roda `jim verificar --raiz exemplos/minimo --contrato CONFIANCA/jim.json --reproduzir`, com `uvx` ou o Jim do venv. Substitua `CONFIANCA/jim.json` pelo caminho da cópia de confiança.

### Modo B — só web

Sem terminal, leia os links raw com a ferramenta web e siga o protocolo à mão. Use a tabela `afirmação | evidência | veredito`, com teto UNPROVEN. Não execute nada nem tente clone, curl, pip ou uvx.

### Modo C — terminal sem rede

A sonda falhou uma vez. Siga o Modo B, com a mesma tabela e teto UNPROVEN. Não insista em clone, uvx, pip ou curl. Se houver um zip anexado, use-o para ler os arquivos. A instalação a partir do zip também precisa de rede.

## Vereditos e códigos de saída

Esta tabela vale para `jim provar` e `jim verificar`. Saída 0 de `--help`, `--version` ou `jim cadeia verificar` (que pode sair 0 sem reprodução) nunca vale como PASS.

| Código | Veredito | Quando se aplica |
|---|---|---|
| 0 | PASS | O gate aceita a evidência íntegra e a reprodução fora do agente confere. |
| 1 | FAIL | A evidência contradiz um critério ou há divergência. |
| 2 | UNKNOWN | Falta informação ou a entrada está malformada. |
| 3 | UNPROVEN | O gate aceitaria, mas falta reprodução ou ela não cobre o fecho. |
| 4 | Erro sem veredito | Erro de uso, de entrada, de I/O ou falha interna. |

UNKNOWN nunca vira PASS. Registre uma nova prova quando houver a evidência que faltava. Sem execução real, nunca diga "pronto". No protocolo manual, indique o que foi apenas inspecionado e use os códigos como referência da CLI.

## Método em cinco passos

1. **Contrato primeiro.** Antes de construir, defina as afirmações verificáveis, os critérios de reprovação e o instrumento que observa cada resultado.
2. **Controle negativo.** Para cada teste, mude um artefato para quebrar o comportamento relevante. O teste precisa rejeitar esse caso pelo critério combinado. No exemplo, trocar `2` por `3` em `valor.txt` muda a saída de `{"dobro": 4}` para `{"dobro": 6}` e o critério deixa de ser atendido.
3. **Prova reproduzível.** No Modo A, rode `jim provar` e guarde o fecho local em `.jim/`. Registre arquivos, comandos, códigos e streams completos. Nos modos B e C, descreva como executar e que resultado se espera, indicando quais observações faltam.
4. **Veredito.** Registre o veredito da prova do agente. No protocolo manual, marque FAIL se a evidência disponível contradiz a afirmação, UNKNOWN se faltam dados para avaliá-la e UNPROVEN se os critérios parecem atendidos sem reprodução.
5. **Tabela final.** Uma linha por afirmação. Cite a evidência ou a lacuna concreta e o veredito correspondente.

| Afirmação | Evidência | Veredito |
|---|---|---|
| O exemplo dobra 2 e o controle negativo é rejeitado. | A leitura de `sonda.py` e `jim.json` sustenta a previsão. Falta observar a execução. | UNPROVEN |

## Links raw

- [README.md](https://raw.githubusercontent.com/XAKCN/Jim/main/README.md)
- [AGENTS.md](https://raw.githubusercontent.com/XAKCN/Jim/main/AGENTS.md)
- [CONTRATO.md](https://raw.githubusercontent.com/XAKCN/Jim/main/CONTRATO.md)
- [exemplos/minimo/](https://raw.githubusercontent.com/XAKCN/Jim/main/exemplos/minimo/LEIAME.md)
  - [jim.json](https://raw.githubusercontent.com/XAKCN/Jim/main/exemplos/minimo/jim.json)
  - [sonda.py](https://raw.githubusercontent.com/XAKCN/Jim/main/exemplos/minimo/sonda.py)
  - [valor.txt](https://raw.githubusercontent.com/XAKCN/Jim/main/exemplos/minimo/valor.txt)

Não carregue `tests/diferencial/casos/` no contexto. São cerca de 400 KB de JSON gerados.

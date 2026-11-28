# Exemplo mínimo

Com o Jim instalado, rode a partir da raiz de uma cópia confiável do repositório.

`jim provar` lê por padrão o `RAIZ/jim.json` do candidato. Quem verifica fornece em `--contrato` uma cópia de confiança independente do agente. Neste exemplo, é o `exemplos/minimo/jim.json` do repositório confiável. Um fecho feito com critérios enfraquecidos pelo agente só passa pela conferência do contrato se a cópia de confiança tiver os mesmos bytes.

Quem verifica roda a reprodução fora do agente. Ao verificar um candidato recebido, aponte `--raiz` para a cópia recebida e `--contrato` para o arquivo do repositório confiável.

| Comando | Código | Veredito |
|---|---|---|
| `jim provar --raiz exemplos/minimo` | 3 | UNPROVEN |
| `jim verificar --raiz exemplos/minimo --contrato exemplos/minimo/jim.json` | 3 | UNPROVEN |
| `jim verificar --raiz exemplos/minimo --contrato exemplos/minimo/jim.json --reproduzir` | 0 | PASS |

O exemplo mantém o fecho e a cadeia em `.jim/` sob a raiz do candidato.

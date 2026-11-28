# Exemplo mínimo

Com o Jim instalado, rode a partir da raiz do repositório.

| Comando | Código | Veredito |
|---|---|---|
| `jim provar --raiz exemplos/minimo` | 3 | UNPROVEN |
| `jim verificar --raiz exemplos/minimo` | 3 | UNPROVEN |
| `jim verificar --raiz exemplos/minimo --reproduzir` | 0 | PASS |

Quem verifica roda a reprodução fora do agente. O exemplo mantém o fecho e a cadeia em `.jim/`.

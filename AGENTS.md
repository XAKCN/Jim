# Agentes no Jim

O loop obriga o agente a **PROVAR**. O agente registra a evidência com
`jim provar`. A aprovação cabe a quem verifica e reproduz fora do agente.
PASS exige reprodução e saída 0 de `jim verificar --reproduzir`.
Sem reprodução, uma prova que atende aos critérios fica **UNPROVEN**.

Leia [CONTRATO.md](CONTRATO.md) e os arquivos congelados em `contract/`.
O código Python prepara arquivos e observa subprocessos. O núcleo Rust
decide o veredito e valida os dados de decisão. Use `_core.json_estrito`
para ler esses dados e preserve os streams completos.

Rode `PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -v` depois de
instalar o Jim. O CI compara os casos versionados com o núcleo instalado.
Mantenha fechos e cadeias locais em `.jim/`, fora do Git.

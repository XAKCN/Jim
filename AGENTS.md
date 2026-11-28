# Agentes no Jim

O loop obriga o agente a **PROVAR**. O agente registra a evidência com
`jim provar` e nunca declara PASS. O agente reporta no máximo **UNPROVEN**,
mesmo que observe saída 0. PASS só vale quando quem verifica reproduz fora
do agente e obtém saída 0 de `jim verificar --contrato CONFIANCA/jim.json --reproduzir`.

Leia [CONTRATO.md](CONTRATO.md) e os arquivos congelados em `contract/`.
O código Python prepara arquivos e observa subprocessos. O núcleo Rust
decide o veredito e valida os dados de decisão. Use `_core.json_estrito`
para ler esses dados e preserve os streams completos.

Rode `PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -v` depois de
instalar o Jim. O CI compara os casos versionados com o núcleo instalado.
Mantenha fechos e cadeias locais em `.jim/`, fora do Git.

Ao receber `gauntlet` junto ao link do Jim, leia [GAUNTLET.md](GAUNTLET.md).
Detecte o ambiente e responda na primeira linha `gauntlet armado: modo X (descrição curta); teto Y`.
No Modo A, use `gauntlet armado: modo A (terminal com rede); teto UNPROVEN até reprodução fora do agente`.
Nos modos B e C, use `teto UNPROVEN`.

- **Modo A (terminal com rede).** A sonda única `curl -sS -m 5 -I https://github.com` respondeu, então use o Jim.
- **Modo B (só web).** Sem terminal, leia os links raw e aplique à mão `afirmação | evidência | veredito`.
- **Modo C (terminal sem rede).** Se a sonda falhar, siga o Modo B sem repetir a sonda ou tentar instalar.

No Modo A, execute na raiz do repositório.

```sh
uvx --from git+https://github.com/XAKCN/Jim jim provar --raiz exemplos/minimo
```

Um venv evita o bloqueio do pip do sistema pelo PEP 668.

```sh
python3 -m venv .venv && .venv/bin/pip install git+https://github.com/XAKCN/Jim
.venv/bin/jim provar --raiz exemplos/minimo
```

Não carregue tests/diferencial/casos/ no contexto: são ~400 KB de JSON gerados.

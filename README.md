# Jim

O loop obriga o agente a **PROVAR**. O agente executa os instrumentos e
registra a evidência. Quem verifica reproduz a execução fora do agente.
PASS só sai quando a prova confere e a reprodução confirma a evidência.
Sem reprodução, uma prova que atende aos critérios fica **UNPROVEN**.

## Instalação

Use CPython 3.9 ou superior e Rust 1.85 ou superior.

```sh
pip install git+https://github.com/XAKCN/Jim
```

O uv também pode instalar e executar o Jim em um comando.

```sh
uvx --from git+https://github.com/XAKCN/Jim jim --version
```

## Loop de prova

1. Defina `jim.json` com os arquivos do candidato, critérios, instrumentos
   Python e saídas esperadas. Use controles negativos para critérios críticos.
2. O agente roda `jim provar`. O Jim executa em cópias temporárias e grava
   o fecho e a cadeia em `.jim/`. Uma prova que atende aos critérios sai com 3.
3. Quem verifica roda `jim verificar --reproduzir` com o contrato de confiança.
   O Jim confere o fecho antes de executar os instrumentos da cópia.
   O código 0 confirma PASS. Use instrumentos de confiança, pois a execução
   usa subprocessos com as permissões do usuário.

O [contrato](CONTRATO.md) detalha o formato, a cadeia e os códigos de saída.
Com o Jim instalado, rode `python -B demo/rodar.py` para ver a recusa de um
falso PASS e a reprodução de um candidato honesto.

Jim é de XAKCN e usa a licença [MIT](LICENSE).

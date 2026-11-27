"""Cenário sintético usado na CLI e na geração diferencial."""

import json

CAMINHO = "sonda.py"
SONDA = """import json
from pathlib import Path

valor = int(Path("valor.txt").read_text(encoding="utf-8"))
print(json.dumps({"dobro": valor * 2}))
"""
CONTRATO = {
    "esquema": "jim-contrato/1",
    "versao": "1",
    "data_referencia": "2026-01-01",
    "arquivos": ["*.py", "*.txt"],
    "criterios": {
        "dobro": {
            "comando": [CAMINHO],
            "aceitos": ["execucao"],
            "severidade": "critica",
            "reprova": "o dobro de 2 deve ser 4",
            "esperado": {"dobro": 4},
            "controles_negativos": [{"valor.txt": "3\n"}],
        },
    },
}


def contrato_bytes(contrato):
    return (json.dumps(contrato, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")


def preparar(raiz, contrato=CONTRATO, sonda=SONDA):
    (raiz / "jim.json").write_bytes(contrato_bytes(contrato))
    (raiz / CAMINHO).write_text(sonda, encoding="utf-8")
    (raiz / "valor.txt").write_bytes(b"2\n")

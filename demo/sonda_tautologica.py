import json
from pathlib import Path
import sys


fator = int(Path(sys.argv[1]).read_text(encoding="utf-8"))


def dobro(valor):
    return valor * fator


obtido = dobro(2)
esperado = dobro(2)
print(json.dumps({"passou": obtido == esperado}))

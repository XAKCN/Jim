import json
from pathlib import Path

valor = int(Path("valor.txt").read_text(encoding="utf-8"))
print(json.dumps({"dobro": valor * 2}))

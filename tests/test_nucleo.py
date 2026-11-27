import unittest

from jim import _core


class NucleoTest(unittest.TestCase):
    def test_montagem_respeita_limite_do_leitor_estrito(self):
        contrato = b"""{
            "esquema": "jim-contrato/1", "versao": "1", "data_referencia": "2026-01-01",
            "arquivos": ["t.py"], "criterios": {"t": {
                "comando": ["t.py"], "aceitos": ["execucao"], "severidade": "alta",
                "reprova": "falha", "regra": "todos_aprovados"
            }}
        }"""
        arquivos = {"t.py": "a" * 64}
        sha = _core.sha256_hex(contrato)

        def montar(niveis):
            stdout = ('{"casos":1,"aprovados":1,"falhas":[],"extra":'
                      + "[" * niveis + "0" + "]" * niveis + "}")
            execucao = {"criterio": "t", "comando": ["python", "t.py"],
                        "exit_code": 0, "stdout": stdout, "stderr": "", "duracao_s": 0.5}
            return _core.montar_fecho(contrato, "jim.json", [execucao], arquivos, arquivos, sha, {})

        fecho = montar(60)
        self.assertEqual(_core.verificar_fecho(fecho["dados"], contrato, arquivos)["veredito"],
                         "UNPROVEN")
        with self.assertRaisesRegex(_core.EntradaInvalida, "^R-5.16: mais de 64 níveis"):
            montar(61)

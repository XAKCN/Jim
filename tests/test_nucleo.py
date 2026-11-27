import unittest

from jim import _core

from .diferencial.formato import carregar_casos, decodificar


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

    def test_derivacao_recusa_execucao_gravada_malformada(self):
        caso = next(c for c in carregar_casos() if c["funcao"] == "derivar_registros")
        args = decodificar(caso["args"])
        execucao, = args["execucoes"]
        self.assertTrue(_core.derivar_registros(**args)["avaliacoes"])
        del execucao["duracao_s"]
        with self.assertRaisesRegex(_core.EntradaInvalida, "^R-2.1: execução gravada malformada"):
            _core.derivar_registros(**args)
        execucao["duracao_s"] = True
        with self.assertRaisesRegex(_core.EntradaInvalida, "^R-2.1: execução gravada malformada"):
            _core.derivar_registros(**args)

    def test_contrato_gate_recusa_dependencias_que_nao_sao_caminhos(self):
        caso = next(c for c in carregar_casos() if c["funcao"] == "contrato_gate")
        contrato = decodificar(caso["args"])["contrato"]
        criterio, = contrato["criterios"].values()
        criterio.pop("controles_negativos")
        for dependencia in (1, "./sonda.py", "contrato"):
            with self.subTest(dependencia=dependencia):
                criterio["depende_de"] = [dependencia]
                with self.assertRaisesRegex(_core.EntradaInvalida, "^R-2.12:"):
                    _core.validar_contrato(contrato, ["sonda.py", "valor.txt"])
                with self.assertRaisesRegex(_core.EntradaInvalida, "^R-2.12:"):
                    _core.contrato_gate(contrato, "id")

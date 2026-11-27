"""Espelho dos casos obrigatórios de SEMANTICA §7."""

import re

_ITENS = {
    "H": (
        "vetor §2.2", "vetor §2.6", "vetor §6.1", "ordem maiúscula/é/e+U+0301",
        "True/1/1.0", "0.0/-0.0", "2^53-1 int", "2^53-1 float", "2^53 int",
        "2^53 float", "9007199254740991.5", "17 dígitos", "1e400", "1e-400",
        "NaN", "Infinity", "-Infinity", "número antes de repetição", "chave a repetida",
        "par escapado e direto", "substituto isolado", "chave substituta repetida",
        "profundidade 64", "profundidade 65", "colchetes em texto fechado",
        "colchetes em texto sem fecho", "BOM", "controle cru", "dado extra",
        "repr aspas simples", "repr aspas duplas", "repr ambas", "repr tab",
        "repr U+0001", "repr U+007F",
    ),
    "K": (
        "íntegra e ancorada", "sem âncora", "vazia", "sem LF final",
        "UTF-8 antes de parse", "chave a mais", "chave a menos", "seq true",
        "seq 1.0", "seq texto", "anterior errado", "evento lista",
        "quebrada com âncora", "âncora n bool", "âncora cabeça int", "âncora lista",
        "âncora n divergente", "âncora cabeça divergente", "remoção última linha",
        "CRLF", "append idêntico", "append quebrada", "eventos sem âncora",
    ),
    "G": (
        "E1", "E2", "E3", "E4a", "E4b", "E4c", "E4d",
        "1a", "1b", "1c", "1d", "2a", "2b", "2c", "2d", "2e", "2f", "2g",
        "2h", "2h'", "2h''", "2i", "2j", "2k", "2l", "2l'", "3",
        "4a", "4b", "5a", "5b", "5c", "5d bloqueado por 5b", "6",
        "I1", "I2", "I3", "I4", "I5", "I6",
        "C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10",
        "precedência data", "precedência correntes", "precedência primeiro",
        "precedência segundo", "precedência integração", "registro extra inválido",
        "premissas null", "premissa candidato", "premissas vazias",
        "ordem_falhas inválida e empate", "obrigatório repetido", "obrigatório inválido",
        "integração 4.2", "integração true", "candidato fora de correntes",
        "família x U+001C e X", "família não ASCII", "Humano",
        "data 2026-02-30", "data 2026-1-01", "data 0000-01-01",
        "véspera", "dia", "dia seguinte", "instrumento com espaços", "instrumento vazio",
        "versao_contrato ausente", "versao_contrato null", "versao_contrato int",
        "versao_contrato lista", "avaliações em pares", "exige_humano null",
        "duas dependências ausentes", "ordem de inserção R-2.12", "correntes lista",
        "hoje malformado",
    ),
    "D": (
        "PASS com esperado", "todos_aprovados", "crítica sem controles",
        "controle com stderr", "controle com erro", "formato bool e int",
        "lixo depois da última linha", "CRLF", "U+001C", "U+00A0 antes de objeto",
        "última inválida sem recuo", "exit diferente de zero", "saída None",
        "arquivo mudado", "arquivo apagado", "contrato mudado", "sem_sobreviventes",
    ),
    "J": (
        "montar gate PASS", "sem reprodução", "reprodução igual", "exit divergente",
        "critério faltando", "cópia diferente", "bytes não canônicos",
        "stdout adulterado", "saída adulterada", "âncora divergente",
        "árvore atual mudou", "contrato trocado", "PASS gravado", "stderr de controle",
        "campo informativo sem novo selo", "controles fora do contrato",
        "exit de controle divergente", "saída de controle divergente",
    ),
}

LIMITES = {f"{grupo}: {item}" for grupo, itens in _ITENS.items() for item in itens}
GRUPOS = dict(H="hash", K="cadeia", G="gate", D="derivacao", J="fecho")
CAMINHO_PRIVADO = re.compile(
    r"""(?:^|[\s"'])/(?:home|Users|root|tmp|private/tmp|var/tmp|workspace|run/user)(?:/|[\s"'])"""
    r"""|[A-Za-z]:[\\/]+(?:Users|Temp)[\\/]""", re.IGNORECASE,
)

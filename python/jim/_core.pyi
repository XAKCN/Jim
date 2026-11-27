"""Interface CONGELADA de `jim._core` — Jim 0.1.0, contrato C0.

Fonte da verdade: este arquivo + contract/SEMANTICA.md (§n abaixo = seção de lá).
Nome, parâmetro, padrão, tipo de retorno ou exceção só muda com nova versão
do contrato (CONTRATO.md §6). O pacote Python copia este arquivo, sem edição,
para python/jim/_core.pyi.

Regras gerais (valem para toda função):
- G1. Parâmetro anotado `Json` passa pelo domínio J-D (§1.1): só None, bool,
  int, float, str, list e dict com chaves str, tipos EXATOS (subclasse, tuple,
  set, bytes, date etc. são recusados), |número| <= 2**53 - 1, float finito,
  str sem substituto UTF-16 isolado, até 64 níveis. Fora dele: EntradaInvalida.
- G2. Parâmetro anotado `bytes`, `str`, `List[str]` ou `Dict[str, str]` com
  outro tipo: EntradaInvalida "J-D0: ..." (nunca TypeError do PyO3, nunca
  conversão implícita: bytearray/memoryview/Path não valem).
- G3. Nenhuma função devolve PASS por omissão. Erro interno, pânico Rust ou
  caso não previsto viram exceção (EntradaInvalida ou NaoImplementado);
  nunca um dict com "PASS". Pânico não pode atravessar a fronteira como PASS.
- G4. Retornos são objetos Python novos; nada da entrada é devolvido por
  referência. dict preserva a ordem descrita em §1.3.
- G5. Toda mensagem de erro e todo `motivo` começa pelo código do requisito:
  "R-x.y: " (herdado da v5-alpha) ou "J-..: " (próprio do Jim).
- G6. Sem estado global, sem rede, sem variáveis de ambiente. Só
  `inventariar` lê o sistema de arquivos; nenhuma função escreve nele.
"""
from typing import Dict, List, Literal, Optional, TypedDict, Union

Json = Union[None, bool, int, float, str, List["Json"], Dict[str, "Json"]]
Resultado = Literal["PASS", "FAIL", "UNKNOWN"]
Veredito = Literal["PASS", "FAIL", "UNKNOWN", "UNPROVEN"]

VERSAO: str            # "0.1.0"
ESQUEMA_CONTRATO: str  # "jim-contrato/1"
ESQUEMA_FECHO: str     # "jim-fecho/1"
GENESE: str            # "0" * 64
IMPLEMENTADO: bool     # False no esqueleto; True só quando TODA função estiver implementada


class EntradaInvalida(ValueError):
    """Entrada fora do domínio ou malformada (equivale ao ValueError da v5-alpha)."""


class NaoImplementado(NotImplementedError):
    """Função ainda não implementada. Mensagem começa por "J-0: "."""


# ---------------------------------------------------------------- tipos de saída

class ResultadoParte(TypedDict):
    resultado: Resultado
    motivo: Optional[str]          # None se e só se resultado == "PASS"


class ResultadoCriterio(TypedDict):
    resultado: Resultado
    motivo: Optional[str]
    severidade: Optional[str]      # "critica"|"alta"|"media"|"baixa" ou None


class SaidaGate(TypedDict):
    veredito: Resultado
    contrato: ResultadoParte
    criterios: Dict[str, ResultadoCriterio]
    integracao: ResultadoParte
    ordem_falhas: List[str]


class Ancora(TypedDict):
    n: int
    cabeca: str                    # SHA-256 hex minúsculo da última linha (ou GENESE)


class VerificacaoCadeia(TypedDict):
    integra: bool
    ancorada: Optional[bool]
    n: int
    cabeca: str
    erro: Optional[str]


class LinhaCadeia(TypedDict):
    linha: bytes                   # bytes a acrescentar, já com o b"\n" final
    ancora: Ancora                 # âncora depois do acréscimo


class Registros(TypedDict):
    avaliacoes: Dict[str, Dict[str, Json]]
    integracao: Dict[str, Json]
    correntes: Dict[str, str]


class ResultadoReproducao(TypedDict):
    resultado: Literal["PASS", "FAIL", "UNPROVEN"]
    motivo: Optional[str]


class FechoMontado(TypedDict):
    fecho: Dict[str, Json]         # documento jim-fecho/1 (§5.1)
    dados: bytes                   # json_canonico(fecho) + b"\n": o que se grava em disco
    sha256: str                    # sha256_hex(dados)
    veredito: Literal["FAIL", "UNKNOWN", "UNPROVEN"]   # nunca "PASS" (§5.2)


class Verificacao(TypedDict):
    veredito: Veredito             # único lugar onde "PASS" final pode aparecer (§5.4)
    motivo: Optional[str]          # None se e só se veredito == "PASS"
    erros: List[str]               # todos os problemas achados, na ordem de §5.3
    fecho_sha256: str              # sha256_hex(fecho_bytes), sempre presente
    candidato: Optional[str]       # "sha256:..." do fecho, se legível
    contrato_sha256: Optional[str]
    gate: Optional[SaidaGate]      # gate_43 recalculado, se chegou lá
    reproducao: Optional[ResultadoReproducao]   # None se não houve reprodução


# ---------------------------------------------------------------- tipos de entrada (do Python)

class Controle(TypedDict):
    alterado: bool
    exit_code: int
    stdout: str                    # stream INTEIRO decodificado em UTF-8 estrito (tools-N7)
    stderr: str


class _ExecucaoBase(TypedDict):
    criterio: str
    comando: List[str]             # ["python", <caminho .py do fecho>, args...]
    exit_code: int                 # 124 em timeout
    stdout: str                    # stream INTEIRO (tools-N7), sem tradução de fim de linha
    stderr: str
    duracao_s: float               # informativo; não entra em decisão


class Execucao(_ExecucaoBase, total=False):
    controles_negativos: List[Controle]   # presente se e só se o critério declara controles


class Reproducao(TypedDict):
    inventario_antes: Dict[str, str]      # inventariar(copia, caminhos do fecho) antes de rodar
    inventario_depois: Dict[str, str]     # idem, depois de todas as execuções
    execucoes: List[Execucao]             # uma por critério, na ordem do contrato


# ---------------------------------------------------------------- §2 bytes, JSON e hash

def sha256_hex(dados: bytes) -> str:
    """SHA-256 dos bytes, 64 caracteres hex minúsculos."""


def json_estrito(dados: bytes) -> Json:
    """Decodifica UTF-8 estrito e faz o parse JSON com as regras de §2.1
    (as de cadeia._carregar da v5-alpha). Violação: EntradaInvalida com a
    mesma mensagem que cadeia._carregar levantaria ("R-5.16: ...")."""


def json_canonico(valor: Json) -> bytes:
    """Serialização canônica de §2.2 (bit a bit igual a json.dumps(valor,
    ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode())."""


def iguais(a: Json, b: Json) -> bool:
    """json_canonico(a) == json_canonico(b) (release.iguais da v5-alpha):
    True, 1 e 1.0 são distintos; 0.0 e -0.0 são distintos; ordem de chaves não importa."""


def repr_py(valor: Json) -> str:
    """repr() do Python para valores J-D (§2.4); usado nos motivos."""


def validar_caminho(caminho: str) -> None:
    """Caminho relativo canônico de §2.5, ou EntradaInvalida "J-C1: ..."."""


def hash_fecho(arquivos: Dict[str, str]) -> str:
    """"sha256:" + hash da árvore {caminho: sha256 hex} (§2.6); igual ao
    "sha256:" + release.hash_arvore da v5-alpha. Vazio, caminho inválido ou
    hash fora de [0-9a-f]{64}: EntradaInvalida."""


def inventariar(raiz: str, caminhos: List[str]) -> Dict[str, str]:
    """{caminho: sha256_hex(bytes)} lendo raiz/caminho (§2.7). Recusa
    (EntradaInvalida) caminho inválido, symlink em qualquer componente,
    saída da raiz, não-arquivo e caminho repetido. Arquivo ausente:
    EntradaInvalida "J-C4: ..." (quem chama decide se ausência é FAIL).
    Symlink na abertura também é recusado (J-C5). Erro de acesso em
    qualquer componente ou abertura segura indisponível: EntradaInvalida
    "J-C7: ..."."""


# ---------------------------------------------------------------- §3 gate (semântica gate43 da v5-alpha)

def avaliar_criterio_43(registro: Json, hoje: Json, aceitos: Json, correntes: Json,
                        familias_aceitas: Json = None, exige_humano: bool = False) -> ResultadoParte:
    """gate43.avaliar_criterio_43 da v5-alpha (§3.2). `hoje` é "AAAA-MM-DD";
    qualquer outro valor (inclusive não-str) é EntradaInvalida R-2.14 (§3.2 E1).
    `exige_humano` omitido vale False; None ou não-bool explícito é
    EntradaInvalida R-2.11. Erros de domínio da v5-alpha (ValueError) são
    EntradaInvalida com o mesmo código."""


def gate_43(contrato: Json, avaliacoes: Json, integracao: Json, hoje: Json,
            correntes: Json) -> SaidaGate:
    """gate43.gate_43 da v5-alpha (§3.3–§3.5). `hoje` como em avaliar_criterio_43.
    `avaliacoes` tem de ser dict
    (pares/iteradores não valem: J-D2). Nunca devolve "UNPROVEN": isso é do
    veredito do Jim (§5.4)."""


# ---------------------------------------------------------------- §4 derivação de registros

def validar_contrato(contrato: Json, arquivos: List[str]) -> None:
    """Contrato jim-contrato/1 (§4.1) contra a lista de caminhos resolvidos
    do fecho. Inválido: EntradaInvalida."""


def contrato_gate(contrato: Json, candidato: str) -> Dict[str, Json]:
    """Contrato no formato de gate_43 (§4.2; release.contrato_gate da v5-alpha).
    Pressupõe validar_contrato; contrato inválido: EntradaInvalida.
    Item de `depende_de` fora de §2.5: EntradaInvalida "R-2.12: ..."."""


def ultima_linha_json(texto: str) -> Json:
    """Última linha JSON-objeto do stdout (§4.3), ou None. Nunca levanta para
    `texto` str."""


def avaliar_saida(saida: Json, criterio: Json) -> bool:
    """Saída do instrumento atende ao critério? (§4.4; release.avaliar_saida,
    só "esperado" e "todos_aprovados"). Regra desconhecida: EntradaInvalida."""


def controles_aprovados(execucao: Json, criterio: Json) -> bool:
    """Controles negativos observaram rejeição válida? (§4.5). `execucao` no
    formato gravado do fecho (§5.1)."""


def derivar_registros(contrato: Json, execucoes: Json, executados: Dict[str, str],
                      atuais: Dict[str, str], contrato_sha256: str,
                      contrato_sha256_depois: str) -> Registros:
    """release.registros da v5-alpha (§4.6): evidências, integração e correntes
    derivadas só do observado. `execucoes` e seus controles devem obedecer
    às chaves e aos tipos do formato gravado (§5.1), ou EntradaInvalida."""


# ---------------------------------------------------------------- §5 fecho e veredito do Jim

def montar_fecho(contrato_bytes: bytes, caminho_contrato: str, execucoes: List[Execucao],
                 arquivos: Dict[str, str], arquivos_depois: Dict[str, str],
                 contrato_sha256_depois: str, ambiente: Dict[str, str]) -> FechoMontado:
    """Monta o documento jim-fecho/1 (§5.1–§5.2) a partir do que o Python
    observou. Contrato, entradas ou bytes finais fora das regras de §2.1:
    EntradaInvalida. O veredito do
    fecho nunca é PASS: no máximo UNPROVEN (a execução foi do agente)."""


def comparar_reproducao(fecho: Json, contrato_bytes: bytes,
                        reproducao: Reproducao) -> ResultadoReproducao:
    """Regras de reprodução P1–P7 (§5.5). `fecho` já validado por
    verificar_fecho; fecho ou reproducao malformados: EntradaInvalida."""


def verificar_fecho(fecho_bytes: bytes, contrato_bytes: bytes, atuais: Dict[str, str],
                    reproducao: Optional[Reproducao] = None,
                    ancora: Optional[str] = None) -> Verificacao:
    """Verificação completa (§5.3) e veredito final do Jim (§5.4).
    V8 confere também a presença e o número de controles negativos do contrato.
    Conteúdo malformado de fecho_bytes/contrato_bytes NÃO levanta: vira
    veredito UNKNOWN com motivo. Levanta EntradaInvalida só por G1/G2 e por
    `atuais`/`reproducao`/`ancora` malformados (vêm do próprio Jim).
    PASS exige reprodução P7; sem `reproducao`, o máximo é UNPROVEN."""


# ---------------------------------------------------------------- §6 cadeia de hashes

def hash_linha(texto: str) -> str:
    """cadeia.hash_linha: SHA-256 hex dos bytes UTF-8 de `texto`."""


def verificar_cadeia(dados: bytes, ancora: Optional[Json] = None) -> VerificacaoCadeia:
    """cadeia.verificar sobre os bytes do arquivo (§6.2). Arquivo inexistente:
    quem chama passa b"". Nunca levanta por conteúdo de `dados` ou `ancora`
    (âncora fora de J-D conta como malformada); só por G2 no tipo de `dados`."""


def eventos_cadeia(dados: bytes, ancora: Json) -> List[Dict[str, Json]]:
    """cadeia.eventos (§6.3): eventos só se íntegra E ancorada; senão
    EntradaInvalida "R-5.16: cadeia não utilizável: ..."."""


def proxima_linha(dados: bytes, evento: Json, ancora: Optional[Json] = None) -> LinhaCadeia:
    """cadeia.acrescentar sem I/O (§6.4): devolve a linha a gravar e a nova
    âncora; recusa (EntradaInvalida) cadeia quebrada ou que não confere com
    a âncora dada. Quem chama grava `linha` com append atômico."""

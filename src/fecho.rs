//! §5 de contract/SEMANTICA.md: documento `jim-fecho/1`, verificação e
//! reprodução fora do agente.
//!
//! PASS final só nasce em `verificar_fecho`, depois de V1–V12 e de
//! `comparar_reproducao` devolver P7 (§5.4). `montar_fecho` nunca devolve PASS.

use crate::gate::{gate_43, Res, SaidaGate};
use crate::hash::{arvore, hash_fecho, sha256_hex, validar_inventario};
use crate::json::{e_hex64, iguais, json_canonico, json_estrito, objeto, repr_texto, Objeto, Valor};
use crate::registros::{
    conferir_execucoes, contrato_gate, derivar_registros, ultima_linha_json, validar_contrato, Inventario,
};
use std::collections::{HashMap, HashSet};

pub const ESQUEMA_FECHO: &str = "jim-fecho/1";
pub const VERSAO: &str = "0.1.0";
const CHAVES_FECHO: [&str; 16] = [
    "esquema", "jim", "contrato", "data_referencia", "candidato", "arquivos", "arquivos_depois", "execucoes",
    "evidencias", "integracao", "correntes", "contrato_gate", "gate", "veredito", "ambiente", "selo",
];
const CHAVES_EXECUCAO: [&str; 6] = ["criterio", "comando", "exit_code", "stdout", "stderr", "duracao_s"];
const CHAVES_CONTROLE: [&str; 4] = ["alterado", "exit_code", "stdout", "stderr"];
const CHAVES_GRAVADAS: [&str; 3] = ["saida", "stdout_sha256", "stderr_sha256"];

/// Veredito de quatro valores do Jim (CONTRATO.md §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Veredito {
    Pass,
    Fail,
    Unknown,
    Unproven,
}

impl Veredito {
    pub fn nome(self) -> &'static str {
        match self {
            Veredito::Pass => "PASS",
            Veredito::Fail => "FAIL",
            Veredito::Unknown => "UNKNOWN",
            Veredito::Unproven => "UNPROVEN",
        }
    }
    /// Veredito do gate quando ele não deu PASS (nunca devolve Pass).
    fn de_gate_nao_pass(r: Res) -> Veredito {
        match r {
            Res::Fail => Veredito::Fail,
            _ => Veredito::Unknown,
        }
    }
}

// ---------------------------------------------------------------- formato

fn so_chaves(o: &Objeto, base: &[&str], extra: &[&str], opcional: Option<&str>) -> bool {
    let mut exigidas: HashSet<&str> = base.iter().chain(extra.iter()).copied().collect();
    let tem_opc = opcional.map(|k| o.contem(k)).unwrap_or(false);
    if tem_opc {
        exigidas.insert(opcional.unwrap_or(""));
    }
    o.len() == exigidas.len() && o.chaves().all(|k| exigidas.contains(k))
}

fn e_numero(v: Option<&Valor>) -> bool {
    matches!(v, Some(Valor::Int(_) | Valor::Float(_)))
}

fn e_texto(v: Option<&Valor>) -> bool {
    matches!(v, Some(Valor::Texto(_)))
}

fn lista_textos(v: Option<&Valor>) -> bool {
    matches!(v, Some(Valor::Lista(l)) if l.iter().all(|x| matches!(x, Valor::Texto(_))))
}

/// Controle: `Controle` do pyi (+ os três campos gravados se `gravado`).
fn controle_ok(c: &Valor, gravado: bool) -> bool {
    let o = match c {
        Valor::Objeto(o) => o,
        _ => return false,
    };
    let extra: &[&str] = if gravado { &CHAVES_GRAVADAS } else { &[] };
    so_chaves(o, &CHAVES_CONTROLE, extra, None)
        && matches!(o.bruto("alterado"), Some(Valor::Bool(_)))
        && matches!(o.bruto("exit_code"), Some(Valor::Int(_)))
        && e_texto(o.bruto("stdout"))
        && e_texto(o.bruto("stderr"))
        && (!gravado || (hex(o.bruto("stdout_sha256")) && hex(o.bruto("stderr_sha256"))))
}

fn hex(v: Option<&Valor>) -> bool {
    matches!(v, Some(Valor::Texto(s)) if e_hex64(s))
}

/// Execução: `Execucao` do pyi (+ os três campos gravados se `gravado`).
fn execucao_ok(e: &Valor, gravado: bool) -> bool {
    let o = match e {
        Valor::Objeto(o) => o,
        _ => return false,
    };
    let extra: &[&str] = if gravado { &CHAVES_GRAVADAS } else { &[] };
    so_chaves(o, &CHAVES_EXECUCAO, extra, Some("controles_negativos"))
        && e_texto(o.bruto("criterio"))
        && lista_textos(o.bruto("comando"))
        && matches!(o.bruto("exit_code"), Some(Valor::Int(_)))
        && e_texto(o.bruto("stdout"))
        && e_texto(o.bruto("stderr"))
        && e_numero(o.bruto("duracao_s"))
        && (!gravado || (hex(o.bruto("stdout_sha256")) && hex(o.bruto("stderr_sha256"))))
        && match o.bruto("controles_negativos") {
            None => true,
            Some(Valor::Lista(l)) => l.iter().all(|c| controle_ok(c, gravado)),
            Some(_) => false,
        }
}

/// {str: str} de um Valor objeto.
fn inventario_de(v: Option<&Valor>) -> Option<Inventario> {
    let o = v?.como_objeto()?;
    o.iter().map(|(k, x)| x.como_texto().map(|s| (k.to_string(), s.to_string()))).collect()
}

fn inventario_valido(v: Option<&Valor>) -> Option<Inventario> {
    let inv = inventario_de(v)?;
    validar_inventario(&inv).ok()?;
    Some(inv)
}

fn inventario_valor(inv: &Inventario) -> Valor {
    let mut o = Objeto::novo();
    for (k, h) in inv {
        o.inserir(k, Valor::texto(h));
    }
    Valor::Objeto(o)
}

fn mapa(inv: &Inventario) -> HashMap<&str, &str> {
    inv.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

/// V4: chaves e tipos de §5.1. Erro = nome do campo.
pub fn validar_formato_fecho(f: &Valor) -> Result<(), String> {
    let o = f.como_objeto().ok_or("fecho")?;
    if !so_chaves(o, &CHAVES_FECHO, &[], None) {
        return Err("chaves".into());
    }
    let falha = |c: &str| Err(c.to_string());
    if !matches!(o.bruto("esquema"), Some(v) if v.e_texto(ESQUEMA_FECHO)) {
        return falha("esquema");
    }
    if !matches!(o.bruto("jim"), Some(v) if v.e_texto(VERSAO)) {
        return falha("jim");
    }
    let ct_ok = match o.bruto("contrato") {
        Some(Valor::Objeto(c)) => {
            so_chaves(c, &["caminho", "sha256", "sha256_depois", "versao"], &[], None)
                && e_texto(c.bruto("caminho"))
                && hex(c.bruto("sha256"))
                && hex(c.bruto("sha256_depois"))
                && e_texto(c.bruto("versao"))
        }
        _ => false,
    };
    if !ct_ok {
        return falha("contrato");
    }
    if !e_texto(o.bruto("data_referencia")) {
        return falha("data_referencia");
    }
    if !e_texto(o.bruto("candidato")) {
        return falha("candidato");
    }
    let arqs = match inventario_valido(o.bruto("arquivos")) {
        Some(a) if !a.is_empty() => a,
        _ => return falha("arquivos"),
    };
    let chaves: HashSet<&str> = arqs.iter().map(|(p, _)| p.as_str()).collect();
    match inventario_valido(o.bruto("arquivos_depois")) {
        Some(d) if d.iter().all(|(p, _)| chaves.contains(p.as_str())) => {}
        _ => return falha("arquivos_depois"),
    }
    if !matches!(o.bruto("execucoes"), Some(Valor::Lista(l)) if l.iter().all(|e| execucao_ok(e, true))) {
        return falha("execucoes");
    }
    for k in ["evidencias", "integracao", "correntes", "contrato_gate", "gate"] {
        if !matches!(o.bruto(k), Some(Valor::Objeto(_))) {
            return falha(k);
        }
    }
    if !matches!(o.bruto("veredito"), Some(Valor::Texto(v)) if ["PASS", "FAIL", "UNKNOWN", "UNPROVEN"].contains(&v.as_str())) {
        return falha("veredito");
    }
    if inventario_de(o.bruto("ambiente")).is_none() {
        return falha("ambiente");
    }
    if !hex(o.bruto("selo")) {
        return falha("selo");
    }
    Ok(())
}

/// Selo (emenda C0-2): sha256_hex(json_canonico(fecho sem "selo")).
pub fn selo_de(f: &Objeto) -> String {
    let mut sem = f.clone();
    sem.remover("selo");
    sha256_hex(&json_canonico(&Valor::Objeto(sem)))
}

fn selar(f: Valor) -> Valor {
    let mut o = f.como_objeto().cloned().unwrap_or_default();
    let s = selo_de(&o);
    o.inserir("selo", Valor::Texto(s));
    Valor::Objeto(o)
}

/// V4b: o selo confere com o resto do documento.
fn conferir_selo(f: &Objeto) -> Result<(), Parada> {
    if matches!(f.bruto("selo"), Some(v) if v.e_texto(&selo_de(f))) {
        Ok(())
    } else {
        Err((Veredito::Fail, "J-V4b: selo do fecho não confere".into()))
    }
}

/// Forma gravada de uma execução: acrescenta `saida` e hashes (M4).
fn gravar(ex: &Valor) -> Valor {
    let mut o = ex.como_objeto().cloned().unwrap_or_default();
    let campos = |o: &mut Objeto| {
        let out = o.bruto("stdout").and_then(|v| v.como_texto()).unwrap_or("").to_string();
        let err = o.bruto("stderr").and_then(|v| v.como_texto()).unwrap_or("").to_string();
        o.inserir("saida", ultima_linha_json(&out).unwrap_or(Valor::Nulo));
        o.inserir("stdout_sha256", Valor::Texto(sha256_hex(out.as_bytes())));
        o.inserir("stderr_sha256", Valor::Texto(sha256_hex(err.as_bytes())));
    };
    if let Some(Valor::Lista(cs)) = o.bruto("controles_negativos").cloned() {
        let novos = cs
            .iter()
            .map(|c| {
                let mut co = c.como_objeto().cloned().unwrap_or_default();
                campos(&mut co);
                Valor::Objeto(co)
            })
            .collect();
        o.inserir("controles_negativos", Valor::Lista(novos));
    }
    campos(&mut o);
    Valor::Objeto(o)
}

/// Execuções de entrada (M3 / P0): formato, uma por critério na ordem,
/// comando e controles conforme o contrato.
fn conferir_entrada(c: &Valor, execucoes: &Valor) -> Result<(), String> {
    let cr = c.como_objeto().and_then(|o| o.get("criterios")).and_then(|v| v.como_objeto()).ok_or("R-2.8: critérios ausentes ou reservados")?;
    let exs = execucoes.como_lista().ok_or("J-M3: execuções malformadas")?;
    for (i, e) in exs.iter().enumerate() {
        if !execucao_ok(e, false) {
            return Err(format!("J-M3: execução malformada: #{}", i + 1));
        }
    }
    conferir_execucoes_sem_stream(cr, exs)?;
    conferir_controles_do_contrato(cr, exs).map_err(|e| format!("J-M3: {e}"))
}

fn conferir_controles_do_contrato(cr: &Objeto, exs: &[Valor]) -> Result<(), String> {
    for (e, (nome, crit)) in exs.iter().zip(cr.iter()) {
        if controles(crit).map(Vec::len) != controles(e).map(Vec::len) {
            return Err(format!("controles negativos diferem do contrato: {nome}"));
        }
    }
    Ok(())
}

fn conferir_execucoes_sem_stream(cr: &Objeto, exs: &[Valor]) -> Result<(), String> {
    if exs.len() != cr.len()
        || exs.iter().zip(cr.chaves()).any(|(e, n)| !matches!(e.como_objeto().and_then(|o| o.bruto("criterio")), Some(v) if v.e_texto(n)))
    {
        return Err("R-2.1: execuções diferem dos critérios".into());
    }
    for (e, (_, crit)) in exs.iter().zip(cr.iter()) {
        let mut esperado = vec![Valor::texto("python")];
        if let Some(Valor::Lista(l)) = crit.como_objeto().and_then(|x| x.bruto("comando")) {
            esperado.extend(l.iter().cloned());
        }
        let ok = matches!(e.como_objeto().and_then(|o| o.bruto("comando")), Some(v) if iguais(v, &Valor::Lista(esperado)));
        if !ok {
            return Err("R-2.1: instrumento gravado diverge".into());
        }
    }
    Ok(())
}

fn data_referencia(c: &Valor) -> Valor {
    c.como_objeto().and_then(|o| o.bruto("data_referencia")).cloned().unwrap_or(Valor::Nulo)
}

fn calcular_gate(c: &Valor, gravadas: &Valor, antes: &Inventario, depois: &Inventario, hc: &str, hc_depois: &str, candidato: &str)
    -> Result<(crate::registros::Registros, Valor, SaidaGate), String> {
    let r = derivar_registros(c, gravadas, antes, depois, hc, hc_depois)?;
    let cg = contrato_gate(c, candidato)?;
    let g = gate_43(&cg, &r.avaliacoes, &r.integracao, &data_referencia(c), &r.correntes)?;
    Ok((r, cg, g))
}

/// Veredito do fecho (M6): UNPROVEN se o gate deu PASS; nunca PASS.
fn veredito_do_fecho(g: &SaidaGate) -> Veredito {
    match g.veredito {
        Res::Pass => Veredito::Unproven,
        r => Veredito::de_gate_nao_pass(r),
    }
}

// ---------------------------------------------------------------- §5.2

#[derive(Clone, Debug)]
pub struct FechoMontado {
    pub fecho: Valor,
    pub dados: Vec<u8>,
    pub sha256: String,
    pub veredito: Veredito,
}

/// §5.2 `montar_fecho`.
#[allow(clippy::too_many_arguments)]
pub fn montar_fecho(
    contrato_bytes: &[u8],
    caminho_contrato: &str,
    execucoes: &Valor,
    arquivos: &Inventario,
    arquivos_depois: &Inventario,
    contrato_sha256_depois: &str,
    ambiente: &Inventario,
) -> Result<FechoMontado, String> {
    // M1
    let c = json_estrito(contrato_bytes)?;
    let hc = sha256_hex(contrato_bytes);
    // M2
    let chaves: Vec<String> = arquivos.iter().map(|(p, _)| p.clone()).collect();
    validar_contrato(&c, &chaves)?;
    let candidato = hash_fecho(arquivos)?;
    validar_inventario(arquivos_depois)?;
    let conj: HashSet<&str> = chaves.iter().map(|s| s.as_str()).collect();
    for (p, _) in arquivos_depois {
        if !conj.contains(p.as_str()) {
            return Err(format!("J-F3: arquivo depois fora do fecho: {}", repr_texto(p)));
        }
    }
    if !e_hex64(contrato_sha256_depois) {
        return Err("J-F2: hash inválido em 'contrato_sha256_depois'".into());
    }
    // M3, M4
    conferir_entrada(&c, execucoes)?;
    let gravadas = Valor::Lista(execucoes.como_lista().map(|l| l.iter().map(gravar).collect()).unwrap_or_default());
    // M5
    let (r, cg, g) = calcular_gate(&c, &gravadas, arquivos, arquivos_depois, &hc, contrato_sha256_depois, &candidato)?;
    // M6
    let veredito = veredito_do_fecho(&g);
    let co = c.como_objeto().ok_or("J-K1: contrato deve ser objeto")?;
    let fecho = objeto(vec![
        ("esquema", Valor::texto(ESQUEMA_FECHO)),
        ("jim", Valor::texto(VERSAO)),
        (
            "contrato",
            objeto(vec![
                ("caminho", Valor::texto(caminho_contrato)),
                ("sha256", Valor::texto(&hc)),
                ("sha256_depois", Valor::texto(contrato_sha256_depois)),
                ("versao", co.bruto("versao").cloned().unwrap_or(Valor::Nulo)),
            ]),
        ),
        ("data_referencia", data_referencia(&c)),
        ("candidato", Valor::texto(&candidato)),
        ("arquivos", inventario_valor(arquivos)),
        ("arquivos_depois", inventario_valor(arquivos_depois)),
        ("execucoes", gravadas),
        ("evidencias", r.avaliacoes),
        ("integracao", r.integracao),
        ("correntes", r.correntes),
        ("contrato_gate", cg),
        ("gate", g.para_valor()),
        ("veredito", Valor::texto(veredito.nome())),
        ("ambiente", inventario_valor(ambiente)),
    ]);
    let fecho = selar(fecho);
    let mut dados = json_canonico(&fecho);
    dados.push(b'\n');
    json_estrito(&dados)?;
    let sha256 = sha256_hex(&dados);
    Ok(FechoMontado { fecho, dados, sha256, veredito })
}

// ---------------------------------------------------------------- §5.3 / §5.5

#[derive(Clone, Debug)]
pub struct ResultadoReproducao {
    pub resultado: Veredito, // Pass | Fail | Unproven
    pub motivo: Option<String>,
}

impl ResultadoReproducao {
    pub fn para_valor(&self) -> Valor {
        objeto(vec![
            ("resultado", Valor::texto(self.resultado.nome())),
            ("motivo", self.motivo.as_deref().map(Valor::texto).unwrap_or(Valor::Nulo)),
        ])
    }
}

#[derive(Clone, Debug)]
pub struct Verificacao {
    pub veredito: Veredito,
    pub motivo: Option<String>,
    pub fecho_sha256: String,
    pub candidato: Option<String>,
    pub contrato_sha256: Option<String>,
    pub gate: Option<Valor>,
    pub reproducao: Option<ResultadoReproducao>,
}

impl Verificacao {
    pub fn para_valor(&self) -> Valor {
        let op = |s: &Option<String>| s.as_deref().map(Valor::texto).unwrap_or(Valor::Nulo);
        objeto(vec![
            ("veredito", Valor::texto(self.veredito.nome())),
            ("motivo", op(&self.motivo)),
            ("erros", Valor::Lista(self.motivo.iter().map(|m| Valor::texto(m)).collect())),
            ("fecho_sha256", Valor::texto(&self.fecho_sha256)),
            ("candidato", op(&self.candidato)),
            ("contrato_sha256", op(&self.contrato_sha256)),
            ("gate", self.gate.clone().unwrap_or(Valor::Nulo)),
            ("reproducao", self.reproducao.as_ref().map(|r| r.para_valor()).unwrap_or(Valor::Nulo)),
        ])
    }
}

/// O que V5–V9 e V11 estabelecem sobre um fecho.
struct Conferido {
    c: Valor,
    hc: String,
    arquivos: Inventario,
    gate: SaidaGate,
}

type Parada = (Veredito, String);

/// V5–V9 sobre um fecho já em forma V4. `info` recebe hc e o gate recalculado.
fn conferir_v5_v9(f: &Objeto, contrato_bytes: &[u8], hc_saida: &mut Option<String>, gate_saida: &mut Option<Valor>)
    -> Result<Conferido, Parada> {
    let falha = |m: &str| (Veredito::Fail, m.to_string());
    let ct = f.bruto("contrato").and_then(|v| v.como_objeto()).ok_or_else(|| falha("J-V4: fecho malformado: contrato"))?;
    // V5
    let c = json_estrito(contrato_bytes).map_err(|e| (Veredito::Unknown, format!("J-V5: contrato ilegível: {e}")))?;
    let hc = sha256_hex(contrato_bytes);
    *hc_saida = Some(hc.clone());
    if !matches!(ct.bruto("sha256"), Some(v) if v.e_texto(&hc)) {
        return Err(falha("R-2.5: contrato diverge do registrado no fecho"));
    }
    // V6
    let arquivos = inventario_de(f.bruto("arquivos")).ok_or_else(|| falha("J-V4: fecho malformado: arquivos"))?;
    let depois = inventario_de(f.bruto("arquivos_depois")).ok_or_else(|| falha("J-V4: fecho malformado: arquivos_depois"))?;
    let chaves: Vec<String> = arquivos.iter().map(|(p, _)| p.clone()).collect();
    validar_contrato(&c, &chaves).map_err(|e| (Veredito::Unknown, format!("J-V6: contrato inválido: {e}")))?;
    let co = c.como_objeto().ok_or_else(|| falha("J-K1: contrato deve ser objeto"))?;
    let versao_ok = matches!((co.bruto("versao"), ct.bruto("versao")), (Some(a), Some(b)) if iguais(a, b));
    let data_ok = matches!((co.bruto("data_referencia"), f.bruto("data_referencia")), (Some(a), Some(b)) if iguais(a, b));
    if !versao_ok || !data_ok {
        return Err(falha("R-2.5: identidade do contrato diverge"));
    }
    // V7
    let candidato = f.bruto("candidato").and_then(|v| v.como_texto()).unwrap_or("");
    match hash_fecho(&arquivos) {
        Ok(h) if h == candidato => {}
        _ => return Err(falha("R-3.13: hash da árvore diverge do candidato")),
    }
    // V8
    let execucoes = f.bruto("execucoes").cloned().unwrap_or(Valor::Nulo);
    conferir_execucoes(&c, &execucoes).map_err(|e| (Veredito::Fail, e))?;
    let criterios = co.get("criterios").and_then(Valor::como_objeto)
        .ok_or_else(|| falha("R-2.8: critérios ausentes ou reservados"))?;
    let exs = execucoes.como_lista().ok_or_else(|| falha("R-2.1: execuções diferem dos critérios"))?;
    conferir_controles_do_contrato(criterios, exs).map_err(|e| falha(&format!("J-V8: {e}")))?;
    // V9
    let hc_depois = ct.bruto("sha256_depois").and_then(|v| v.como_texto()).unwrap_or("").to_string();
    let reconstroi = "R-2.21: evidências/integração não se reconstroem";
    let (r, cg, g) = calcular_gate(&c, &execucoes, &arquivos, &depois, &hc, &hc_depois, candidato).map_err(|_| falha(reconstroi))?;
    let gv = g.para_valor();
    *gate_saida = Some(gv.clone());
    let igual = |k: &str, v: &Valor| matches!(f.bruto(k), Some(x) if iguais(x, v));
    if !(igual("evidencias", &r.avaliacoes)
        && igual("integracao", &r.integracao)
        && igual("correntes", &r.correntes)
        && igual("contrato_gate", &cg)
        && igual("gate", &gv))
    {
        return Err(falha(reconstroi));
    }
    if !matches!(f.bruto("veredito"), Some(v) if v.e_texto(veredito_do_fecho(&g).nome())) {
        return Err(falha("J-V9: veredito gravado diverge do recalculado"));
    }
    Ok(Conferido { c, hc, arquivos, gate: g })
}

/// V11: gate recalculado ≠ PASS.
fn conferir_v11(g: &SaidaGate) -> Result<(), Parada> {
    if g.veredito != Res::Pass {
        return Err((Veredito::de_gate_nao_pass(g.veredito), g.primeiro_motivo()));
    }
    Ok(())
}

/// P0: formato de `Reproducao` (pyi).
pub fn validar_reproducao(rep: &Valor) -> Result<(Inventario, Inventario, Vec<Valor>), String> {
    let m = |c: &str| format!("J-P0: reprodução malformada: {c}");
    let o = rep.como_objeto().ok_or_else(|| m("reproducao"))?;
    if !so_chaves(o, &["inventario_antes", "inventario_depois", "execucoes"], &[], None) {
        return Err(m("chaves"));
    }
    let antes = inventario_valido(o.bruto("inventario_antes")).ok_or_else(|| m("inventario_antes"))?;
    let depois = inventario_valido(o.bruto("inventario_depois")).ok_or_else(|| m("inventario_depois"))?;
    let exs = match o.bruto("execucoes") {
        Some(Valor::Lista(l)) if l.iter().all(|e| execucao_ok(e, false)) => l.clone(),
        _ => return Err(m("execucoes")),
    };
    Ok((antes, depois, exs))
}

fn ex_campo<'a>(e: &'a Valor, k: &str) -> Option<&'a Valor> {
    e.como_objeto().and_then(|o| o.bruto(k))
}

fn controles<'a>(e: &'a Valor) -> Option<&'a Vec<Valor>> {
    ex_campo(e, "controles_negativos").and_then(|v| v.como_lista())
}

fn reproduzir(conf: &Conferido, f: &Objeto, antes: &Inventario, depois: &Inventario, exs: &[Valor]) -> ResultadoReproducao {
    let res = |r: Veredito, m: String| ResultadoReproducao { resultado: r, motivo: Some(m) };
    // P2
    if mapa(antes) != mapa(&conf.arquivos) {
        return res(Veredito::Unproven, "J-P2: cópia da reprodução não é o fecho".into());
    }
    // P3
    let gravadas: Vec<Valor> = f.bruto("execucoes").and_then(|v| v.como_lista()).cloned().unwrap_or_default();
    let p3 = exs.len() == gravadas.len()
        && exs.iter().zip(gravadas.iter()).all(|(r, g)| {
            let mesmo = |k: &str| matches!((ex_campo(r, k), ex_campo(g, k)), (Some(a), Some(b)) if iguais(a, b));
            mesmo("criterio")
                && mesmo("comando")
                && match (controles(r), controles(g)) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.len() == b.len(),
                    _ => false,
                }
        });
    if !p3 {
        return res(Veredito::Unproven, "J-P3: reprodução incompleta ou fora de ordem".into());
    }
    let nome = |g: &Valor| ex_campo(g, "criterio").and_then(|v| v.como_texto()).unwrap_or("").to_string();
    let saida_de = |e: &Valor| ultima_linha_json(ex_campo(e, "stdout").and_then(|v| v.como_texto()).unwrap_or("")).unwrap_or(Valor::Nulo);
    let igual_campo = |a: &Valor, b: &Valor, k: &str| matches!((ex_campo(a, k), ex_campo(b, k)), (Some(x), Some(y)) if iguais(x, y));
    // P4
    for (r, g) in exs.iter().zip(gravadas.iter()) {
        let saida_g = ex_campo(g, "saida").cloned().unwrap_or(Valor::Nulo);
        if !igual_campo(r, g, "exit_code") || !iguais(&saida_de(r), &saida_g) {
            return res(Veredito::Fail, format!("J-P4: reprodução diverge do gravado: {}", nome(g)));
        }
    }
    // P5
    for (r, g) in exs.iter().zip(gravadas.iter()) {
        if let (Some(rc), Some(gc)) = (controles(r), controles(g)) {
            for (i, (a, b)) in rc.iter().zip(gc.iter()).enumerate() {
                let saida_b = ex_campo(b, "saida").cloned().unwrap_or(Valor::Nulo);
                let ok = igual_campo(a, b, "alterado")
                    && igual_campo(a, b, "exit_code")
                    && igual_campo(a, b, "stderr")
                    && iguais(&saida_de(a), &saida_b);
                if !ok {
                    return res(Veredito::Fail, format!("J-P5: controle negativo diverge do gravado: {}#{}", nome(g), i + 1));
                }
            }
        }
    }
    // P6
    let rep_gravadas = Valor::Lista(exs.iter().map(gravar).collect());
    let candidato = format!("sha256:{}", arvore(antes));
    let p6 = "J-P6: gate da reprodução não dá PASS";
    match calcular_gate(&conf.c, &rep_gravadas, antes, depois, &conf.hc, &conf.hc, &candidato) {
        Err(e) => res(Veredito::Fail, format!("{p6}: {e}")),
        Ok((_, _, g)) if g.veredito != Res::Pass => res(Veredito::Fail, format!("{p6}: {}", g.primeiro_motivo())),
        // P7: único PASS de reprodução
        Ok(_) => ResultadoReproducao { resultado: Veredito::Pass, motivo: None },
    }
}

/// §5.5 `comparar_reproducao`. O fecho é reconferido (V4–V9, V11); fecho que
/// não passaria em `verificar_fecho` até V11 é EntradaInvalida.
pub fn comparar_reproducao(f: &Valor, contrato_bytes: &[u8], rep: &Valor) -> Result<ResultadoReproducao, String> {
    let (antes, depois, exs) = validar_reproducao(rep)?;
    validar_formato_fecho(f).map_err(|c| format!("J-P0: fecho malformado: {c}"))?;
    let fo = f.como_objeto().ok_or("J-P0: fecho malformado: fecho")?;
    conferir_selo(fo).map_err(|(_, m)| format!("J-P0: fecho não validado: {m}"))?;
    let (mut a, mut b) = (None, None);
    let conf = conferir_v5_v9(fo, contrato_bytes, &mut a, &mut b).map_err(|(_, m)| format!("J-P0: fecho não validado: {m}"))?;
    conferir_v11(&conf.gate).map_err(|(_, m)| format!("J-P0: fecho não validado: {m}"))?;
    Ok(reproduzir(&conf, fo, &antes, &depois, &exs))
}

/// §5.3 `verificar_fecho`.
pub fn verificar_fecho(
    fecho_bytes: &[u8],
    contrato_bytes: &[u8],
    atuais: &Inventario,
    reproducao: Option<&Valor>,
    ancora: Option<&str>,
) -> Result<Verificacao, String> {
    // V0
    validar_inventario(atuais)?;
    if let Some(a) = ancora {
        if !e_hex64(a) {
            return Err("J-V0: ancora deve ser hex64".into());
        }
    }
    let rep = match reproducao {
        Some(r) => Some(validar_reproducao(r)?),
        None => None,
    };
    let mut v = Verificacao {
        veredito: Veredito::Unknown,
        motivo: None,
        fecho_sha256: sha256_hex(fecho_bytes),
        candidato: None,
        contrato_sha256: None,
        gate: None,
        reproducao: None,
    };
    let parar = |mut v: Verificacao, ver: Veredito, m: String| {
        v.veredito = ver;
        v.motivo = Some(m);
        Ok(v)
    };
    // V1
    if let Some(a) = ancora {
        if a != v.fecho_sha256 {
            return parar(v, Veredito::Fail, "J-V1: fecho não confere com a âncora externa".into());
        }
    }
    // V2
    let f = match json_estrito(fecho_bytes) {
        Ok(f) => f,
        Err(e) => return parar(v, Veredito::Unknown, format!("J-V2: fecho ilegível: {e}")),
    };
    v.candidato = f.como_objeto().and_then(|o| o.bruto("candidato")).and_then(|c| c.como_texto()).map(String::from);
    // V3
    let mut canon = json_canonico(&f);
    canon.push(b'\n');
    if canon != fecho_bytes {
        return parar(v, Veredito::Unknown, "J-V3: fecho fora da forma canônica".into());
    }
    // V4
    if let Err(c) = validar_formato_fecho(&f) {
        return parar(v, Veredito::Unknown, format!("J-V4: fecho malformado: {c}"));
    }
    let fo = match f.como_objeto() {
        Some(o) => o,
        None => return parar(v, Veredito::Unknown, "J-V4: fecho malformado: fecho".into()),
    };
    // V4b
    if let Err((ver, m)) = conferir_selo(fo) {
        return parar(v, ver, m);
    }
    // V5–V9
    let (mut hc, mut gate) = (None, None);
    let r = conferir_v5_v9(fo, contrato_bytes, &mut hc, &mut gate);
    v.contrato_sha256 = hc;
    v.gate = gate;
    let conf = match r {
        Ok(c) => c,
        Err((ver, m)) => return parar(v, ver, m),
    };
    // V10
    let fa: HashSet<&str> = conf.arquivos.iter().map(|(p, _)| p.as_str()).collect();
    let aa: HashSet<&str> = atuais.iter().map(|(p, _)| p.as_str()).collect();
    if fa != aa || fa.len() != atuais.len() {
        return parar(v, Veredito::Fail, "R-3.13: arquivos do contrato diferem do fecho".into());
    }
    let ma = mapa(atuais);
    let mut caminhos: Vec<&(String, String)> = conf.arquivos.iter().collect();
    caminhos.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    for (p, h) in caminhos {
        if ma.get(p.as_str()) != Some(&h.as_str()) {
            return parar(v, Veredito::Fail, format!("R-2.21: árvore atual diverge do fecho: {}", repr_texto(p)));
        }
    }
    // V11
    if let Err((ver, m)) = conferir_v11(&conf.gate) {
        return parar(v, ver, m);
    }
    // V12
    let (antes, depois, exs) = match rep {
        None => return parar(v, Veredito::Unproven, "J-P1: PASS do gate sem reprodução externa".into()),
        Some(r) => r,
    };
    // V13
    let rr = reproduzir(&conf, fo, &antes, &depois, &exs);
    v.reproducao = Some(rr.clone());
    match rr.resultado {
        Veredito::Pass if rr.motivo.is_none() => {
            // §5.4: único caminho para PASS final.
            v.veredito = Veredito::Pass;
            v.motivo = None;
            Ok(v)
        }
        Veredito::Fail => parar(v, Veredito::Fail, rr.motivo.unwrap_or_else(|| "J-P6: reprodução sem motivo".into())),
        _ => parar(v, Veredito::Unproven, rr.motivo.unwrap_or_else(|| "J-P3: reprodução sem motivo".into())),
    }
}

// ---------------------------------------------------------------- testes

#[cfg(test)]
mod testes {
    use super::*;
    use crate::gate::testes::j;
    use crate::registros::testes::{ctl, inv, H1, H2};

    const SAIDA_OK: &str = "linha de log\n{\"ok\": true}\n";

    fn contrato_bytes(extra: &str) -> Vec<u8> {
        format!(
            r#"{{"esquema": "jim-contrato/1", "versao": "v1", "data_referencia": "2026-01-10",
                "arquivos": ["*.py", "dados.txt"],
                "criterios": {{"t": {{"comando": ["t.py", "--x"], "aceitos": ["execucao"], "severidade": "alta",
                                      "reprova": "falha", "esperado": {{"ok": true}}{extra}}}}}}}"#
        )
        .into_bytes()
    }

    fn arquivos() -> Inventario {
        inv(&[("t.py", H1), ("dados.txt", H2)])
    }

    fn execucao(exit: i64, stdout: &str, controles: Option<Vec<Valor>>) -> Valor {
        let mut o = Objeto::novo();
        o.inserir("criterio", Valor::texto("t"));
        o.inserir("comando", j(r#"["python", "t.py", "--x"]"#));
        o.inserir("exit_code", Valor::Int(exit));
        o.inserir("stdout", Valor::texto(stdout));
        o.inserir("stderr", Valor::texto(""));
        o.inserir("duracao_s", Valor::Float(0.25));
        if let Some(c) = controles {
            o.inserir("controles_negativos", Valor::Lista(c));
        }
        Valor::Objeto(o)
    }

    fn controle_entrada(stdout: &str, stderr: &str) -> Valor {
        objeto(vec![
            ("alterado", Valor::Bool(true)),
            ("exit_code", Valor::Int(1)),
            ("stdout", Valor::texto(stdout)),
            ("stderr", Valor::texto(stderr)),
        ])
    }

    fn montar(cb: &[u8], exs: Vec<Valor>) -> FechoMontado {
        let amb = inv(&[("python", "3.x")]);
        let hc = sha256_hex(cb);
        montar_fecho(cb, "jim.json", &Valor::Lista(exs), &arquivos(), &arquivos(), &hc, &amb).unwrap()
    }

    fn rep(exs: Vec<Valor>) -> Valor {
        objeto(vec![
            ("inventario_antes", inventario_valor(&arquivos())),
            ("inventario_depois", inventario_valor(&arquivos())),
            ("execucoes", Valor::Lista(exs)),
        ])
    }

    fn verif(m: &FechoMontado, cb: &[u8], r: Option<&Valor>) -> Verificacao {
        verificar_fecho(&m.dados, cb, &arquivos(), r, None).unwrap()
    }

    fn nao_pass(v: &Verificacao, ver: Veredito, prefixo: &str) {
        assert_ne!(v.veredito, Veredito::Pass, "PASS indevido: {prefixo}");
        assert_eq!(v.veredito, ver, "{:?}", v.motivo);
        let m = v.motivo.as_deref().unwrap();
        assert!(m.starts_with(prefixo), "{m} !~ {prefixo}");
        assert_eq!(v.para_valor().como_objeto().unwrap().get("erros").unwrap().como_lista().unwrap().len(), 1);
    }

    /// Reescreve um campo do fecho e devolve bytes canônicos.
    fn adulterar(m: &FechoMontado, f: impl Fn(&mut Objeto)) -> Vec<u8> {
        let mut o = m.fecho.como_objeto().unwrap().clone();
        f(&mut o);
        let mut d = json_canonico(&selar(Valor::Objeto(o)));
        d.push(b'\n');
        d
    }

    #[test]
    fn j_fecho_unproven_e_pass_so_com_reproducao() {
        let cb = contrato_bytes("");
        let m = montar(&cb, vec![execucao(0, SAIDA_OK, None)]);
        assert_eq!(m.veredito, Veredito::Unproven);
        assert_eq!(m.sha256, sha256_hex(&m.dados));
        assert!(m.dados.ends_with(b"\n"));
        let ch: Vec<&str> = m.fecho.como_objeto().unwrap().chaves().collect();
        assert_eq!(ch, CHAVES_FECHO);
        // sem reprodução → UNPROVEN
        let v = verif(&m, &cb, None);
        nao_pass(&v, Veredito::Unproven, "J-P1: ");
        assert_eq!(v.candidato.as_deref(), m.fecho.como_objeto().unwrap().get("candidato").unwrap().como_texto());
        assert!(v.gate.is_some() && v.contrato_sha256.is_some());
        // reprodução igual (stdout diferente, mesma saída) → PASS
        let r = rep(vec![execucao(0, "outro log\n{\"ok\":true}", None)]);
        let v = verif(&m, &cb, Some(&r));
        assert_eq!(v.veredito, Veredito::Pass);
        assert!(v.motivo.is_none());
        assert_eq!(v.reproducao.as_ref().unwrap().resultado, Veredito::Pass);
        let pv = v.para_valor();
        assert_eq!(pv.como_objeto().unwrap().get("erros").unwrap().como_lista().unwrap().len(), 0);
        // âncora certa mantém PASS; errada dá FAIL
        let v = verificar_fecho(&m.dados, &cb, &arquivos(), Some(&r), Some(&m.sha256)).unwrap();
        assert_eq!(v.veredito, Veredito::Pass);
        let v = verificar_fecho(&m.dados, &cb, &arquivos(), Some(&r), Some(&"0".repeat(64))).unwrap();
        nao_pass(&v, Veredito::Fail, "J-V1: ");
        // comparar_reproducao direto
        let cr = comparar_reproducao(&m.fecho, &cb, &r).unwrap();
        assert_eq!(cr.resultado, Veredito::Pass);
    }

    #[test]
    fn montagem_respeita_limite_do_leitor_estrito() {
        let cb = String::from_utf8(contrato_bytes("")).unwrap()
            .replace(r#""esperado": {"ok": true}"#, r#""regra": "todos_aprovados""#);
        let cb = cb.as_bytes();
        let montar_com_niveis = |n: usize| {
            let stdout = format!(r#"{{"casos":1,"aprovados":1,"falhas":[],"extra":{}0{}}}"#,
                "[".repeat(n), "]".repeat(n));
            let exs = Valor::Lista(vec![execucao(0, &stdout, None)]);
            montar_fecho(cb, "jim.json", &exs, &arquivos(), &arquivos(), &sha256_hex(cb), &vec![])
        };
        let no_limite = montar_com_niveis(60).unwrap();
        assert_eq!(verif(&no_limite, cb, None).veredito, Veredito::Unproven);
        let erro = match montar_com_niveis(61) {
            Err(erro) => erro,
            Ok(fecho) => panic!("montou {:?}, mas o leitor recusou: {:?}",
                fecho.veredito, verif(&fecho, cb, None).motivo),
        };
        assert_eq!(erro, "R-5.16: mais de 64 níveis de aninhamento");
    }

    #[test]
    fn j_reproducao_divergente() {
        let cb = contrato_bytes("");
        let m = montar(&cb, vec![execucao(0, SAIDA_OK, None)]);
        let casos = vec![
            (rep(vec![execucao(1, SAIDA_OK, None)]), Veredito::Fail, "J-P4: reprodução diverge do gravado: t"),
            (rep(vec![execucao(0, "{\"ok\": false}", None)]), Veredito::Fail, "J-P4: "),
            (rep(vec![]), Veredito::Unproven, "J-P3: "),
            (rep(vec![execucao(0, SAIDA_OK, Some(vec![]))]), Veredito::Unproven, "J-P3: "),
        ];
        for (r, ver, pre) in casos {
            nao_pass(&verif(&m, &cb, Some(&r)), ver, pre);
        }
        // cópia diferente
        let mut r = rep(vec![execucao(0, SAIDA_OK, None)]).como_objeto().unwrap().clone();
        r.inserir("inventario_antes", inventario_valor(&inv(&[("t.py", H2), ("dados.txt", H2)])));
        nao_pass(&verif(&m, &cb, Some(&Valor::Objeto(r.clone()))), Veredito::Unproven, "J-P2: ");
        // árvore mudada durante a reprodução → gate da reprodução não dá PASS
        r.inserir("inventario_antes", inventario_valor(&arquivos()));
        r.inserir("inventario_depois", inventario_valor(&inv(&[("t.py", H1)])));
        nao_pass(&verif(&m, &cb, Some(&Valor::Objeto(r))), Veredito::Fail, "J-P6: gate da reprodução não dá PASS: R-2.10");
        // reprodução malformada (sem stream) → exceção
        let mut e = execucao(0, SAIDA_OK, None).como_objeto().unwrap().clone();
        e.remover("stdout");
        let r = rep(vec![Valor::Objeto(e)]);
        assert!(verificar_fecho(&m.dados, &cb, &arquivos(), Some(&r), None).unwrap_err().starts_with("J-P0: "));
    }

    #[test]
    fn j_integridade() {
        let cb = contrato_bytes("");
        let m = montar(&cb, vec![execucao(0, SAIDA_OK, None)]);
        let boa = rep(vec![execucao(0, SAIDA_OK, None)]);
        let v = |d: &[u8], c: &[u8], at: &Inventario| verificar_fecho(d, c, at, Some(&boa), None).unwrap();
        // bytes não canônicos
        let mut d = m.dados.clone();
        d.insert(1, b' ');
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Unknown, "J-V3: ");
        let mut d = m.dados.clone();
        d.pop();
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Unknown, "J-V3: ");
        nao_pass(&v(b"{", &cb, &arquivos()), Veredito::Unknown, "J-V2: fecho ilegível: R-5.16: JSON inválido");
        nao_pass(&v(b"[]\n", &cb, &arquivos()), Veredito::Unknown, "J-V4: fecho malformado: fecho");
        // um byte trocado no fecho: nunca PASS
        for i in 0..m.dados.len() {
            let mut d = m.dados.clone();
            d[i] ^= 0x01;
            let r = v(&d, &cb, &arquivos());
            assert_ne!(r.veredito, Veredito::Pass, "byte {i}");
            assert!(matches!(r.veredito, Veredito::Fail | Veredito::Unknown));
        }
        // campo informativo trocado sem refazer o selo → FAIL (A8)
        let mut o = m.fecho.como_objeto().unwrap().clone();
        o.inserir("ambiente", j("{\"python\": \"3.y\"}"));
        let mut d = json_canonico(&Valor::Objeto(o));
        d.push(b'\n');
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Fail, "J-V4b: selo do fecho não confere");
        let mut o = m.fecho.como_objeto().unwrap().clone();
        o.remover("selo");
        let mut d = json_canonico(&Valor::Objeto(o));
        d.push(b'\n');
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Unknown, "J-V4: fecho malformado: chaves");
        // stdout adulterado
        let d = adulterar(&m, |o| {
            let mut exs = o.get("execucoes").unwrap().como_lista().unwrap().clone();
            let mut e = exs[0].como_objeto().unwrap().clone();
            e.inserir("stdout", Valor::texto("{\"ok\": true} "));
            exs[0] = Valor::Objeto(e);
            o.inserir("execucoes", Valor::Lista(exs));
        });
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Fail, "J-N7: stream gravado não confere: t");
        // saida adulterada
        let d = adulterar(&m, |o| {
            let mut exs = o.get("execucoes").unwrap().como_lista().unwrap().clone();
            let mut e = exs[0].como_objeto().unwrap().clone();
            e.inserir("saida", j("{\"ok\": 1}"));
            exs[0] = Valor::Objeto(e);
            o.inserir("execucoes", Valor::Lista(exs));
        });
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Fail, "J-N7: ");
        // fecho sem stream → malformado
        let d = adulterar(&m, |o| {
            let mut exs = o.get("execucoes").unwrap().como_lista().unwrap().clone();
            let mut e = exs[0].como_objeto().unwrap().clone();
            e.remover("stderr");
            exs[0] = Valor::Objeto(e);
            o.inserir("execucoes", Valor::Lista(exs));
        });
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Unknown, "J-V4: fecho malformado: execucoes");
        // veredito PASS gravado
        let d = adulterar(&m, |o| o.inserir("veredito", Valor::texto("PASS")));
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Fail, "J-V9: ");
        // gate gravado adulterado
        let d = adulterar(&m, |o| o.inserir("gate", j("{\"veredito\": \"PASS\"}")));
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Fail, "R-2.21: evidências/integração não se reconstroem");
        // evidência adulterada
        let d = adulterar(&m, |o| o.inserir("evidencias", j("{}")));
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Fail, "R-2.21: ");
        // candidato adulterado
        let d = adulterar(&m, |o| o.inserir("candidato", Valor::texto("sha256:00")));
        nao_pass(&v(&d, &cb, &arquivos()), Veredito::Fail, "R-3.13: hash da árvore diverge do candidato");
        // contrato trocado
        let outro = contrato_bytes(", \"timeout_s\": 5");
        nao_pass(&v(&m.dados, &outro, &arquivos()), Veredito::Fail, "R-2.5: contrato diverge do registrado no fecho");
        nao_pass(&v(&m.dados, b"{", &arquivos()), Veredito::Unknown, "J-V5: contrato ilegível: ");
        // árvore atual mudada / arquivo a mais
        nao_pass(&v(&m.dados, &cb, &inv(&[("t.py", H2), ("dados.txt", H2)])), Veredito::Fail, "R-2.21: árvore atual diverge do fecho: 't.py'");
        nao_pass(&v(&m.dados, &cb, &inv(&[("t.py", H1)])), Veredito::Fail, "R-3.13: arquivos do contrato diferem do fecho");
        nao_pass(&v(&m.dados, &cb, &inv(&[("t.py", H1), ("dados.txt", H2), ("x.py", H1)])), Veredito::Fail, "R-3.13: ");
        // V0: entradas do Jim malformadas levantam
        assert!(verificar_fecho(&m.dados, &cb, &inv(&[("./t.py", H1)]), None, None).is_err());
        assert!(verificar_fecho(&m.dados, &cb, &arquivos(), None, Some("x")).is_err());
    }

    #[test]
    fn j_gate_nao_pass_nao_reproduz() {
        let cb = contrato_bytes("");
        let m = montar(&cb, vec![execucao(3, SAIDA_OK, None)]);
        assert_eq!(m.veredito, Veredito::Fail);
        let boa = rep(vec![execucao(3, SAIDA_OK, None)]);
        nao_pass(&verif(&m, &cb, Some(&boa)), Veredito::Fail, "R-2.10: adequado deve ser True");
        // comparar_reproducao recusa fecho cujo gate não dá PASS
        assert!(comparar_reproducao(&m.fecho, &cb, &boa).unwrap_err().starts_with("J-P0: fecho não validado: R-2.10"));
        // UNKNOWN: crítico sem controles
        let cb2 = contrato_bytes("").iter().map(|b| *b).collect::<Vec<u8>>();
        let cb2 = String::from_utf8(cb2).unwrap().replace("\"alta\"", "\"critica\"").into_bytes();
        let m2 = montar(&cb2, vec![execucao(0, SAIDA_OK, None)]);
        assert_eq!(m2.veredito, Veredito::Unknown);
        nao_pass(&verif(&m2, &cb2, Some(&rep(vec![execucao(0, SAIDA_OK, None)]))), Veredito::Unknown, "R-2.9: campo adequado omitido");
    }

    #[test]
    fn j_recusa_controles_fora_do_contrato() {
        let cb = contrato_bytes("");
        let m = montar(&cb, vec![execucao(0, SAIDA_OK, None)]);
        let exec = execucao(0, SAIDA_OK, Some(vec![controle_entrada("{\"ok\": false}", "")]));
        let dados = adulterar(&m, |o| o.inserir("execucoes", Valor::Lista(vec![gravar(&exec)])));
        let reproduzida = rep(vec![exec]);
        let v = verificar_fecho(&dados, &cb, &arquivos(), Some(&reproduzida), None).unwrap();
        nao_pass(&v, Veredito::Fail, "J-V8: controles negativos diferem do contrato: t");
        let fecho = json_estrito(&dados).unwrap();
        assert_eq!(
            comparar_reproducao(&fecho, &cb, &reproduzida).unwrap_err(),
            "J-P0: fecho não validado: J-V8: controles negativos diferem do contrato: t",
        );
    }

    #[test]
    fn j_controle_reproduzido_com_stderr() {
        let cb = contrato_bytes(", \"controles_negativos\": [{\"dados.txt\": \"lixo\"}]");
        let ctl_ok = controle_entrada("{\"ok\": false}", "");
        let m = montar(&cb, vec![execucao(0, SAIDA_OK, Some(vec![ctl_ok.clone()]))]);
        assert_eq!(m.veredito, Veredito::Unproven);
        let boa = rep(vec![execucao(0, SAIDA_OK, Some(vec![ctl_ok]))]);
        assert_eq!(verif(&m, &cb, Some(&boa)).veredito, Veredito::Pass);
        let ruim = rep(vec![execucao(0, SAIDA_OK, Some(vec![controle_entrada("{\"ok\": false}", "Traceback")]))]);
        nao_pass(&verif(&m, &cb, Some(&ruim)), Veredito::Fail, "J-P5: controle negativo diverge do gravado: t#1");
        // controle com stderr já na montagem → UNKNOWN
        let m2 = montar(&cb, vec![execucao(0, SAIDA_OK, Some(vec![controle_entrada("{\"ok\": false}", "x")]))]);
        assert_eq!(m2.veredito, Veredito::Unknown);
        let _ = ctl("", "", true, 0);
    }

    #[test]
    fn j_controle_reproduzido_com_rejeicao_diferente() {
        let cb = String::from_utf8(contrato_bytes(", \"controles_negativos\": [{\"dados.txt\": \"lixo\"}]"))
            .unwrap().replace("\"ok\": true", "\"ok\": 1").into_bytes();
        let com_controle = |c| execucao(0, "{\"ok\": 1}", Some(vec![c]));
        let original = controle_entrada("{\"ok\": 2}", "");
        let mut outro_exit = original.como_objeto().unwrap().clone();
        outro_exit.inserir("exit_code", Valor::Int(0));
        let m = montar(&cb, vec![com_controle(original)]);

        for controle in [Valor::Objeto(outro_exit), controle_entrada("{\"ok\": 3}", "")] {
            let observado = com_controle(controle);
            assert_eq!(montar(&cb, vec![observado.clone()]).veredito, Veredito::Unproven);
            let r = rep(vec![observado]);
            let v = verif(&m, &cb, Some(&r));
            nao_pass(&v, Veredito::Fail, "J-P5: controle negativo diverge do gravado: t#1");
        }
    }

    #[test]
    fn montar_recusas() {
        let cb = contrato_bytes("");
        let amb = inv(&[]);
        let hc = sha256_hex(&cb);
        let mf = |exs: Vec<Valor>, a: &Inventario, d: &Inventario, h: &str| {
            montar_fecho(&cb, "jim.json", &Valor::Lista(exs), a, d, h, &amb)
        };
        let e = || vec![execucao(0, SAIDA_OK, None)];
        assert!(montar_fecho(b"{", "j", &Valor::Lista(e()), &arquivos(), &arquivos(), &hc, &amb).unwrap_err().starts_with("R-5.16"));
        assert_eq!(mf(e(), &inv(&[]), &inv(&[]), &hc).unwrap_err(), "R-3.13: instrumento Python fora da cópia");
        assert!(mf(e(), &arquivos(), &inv(&[("x.py", H1)]), &hc).unwrap_err().starts_with("J-F3: "));
        assert!(mf(e(), &arquivos(), &arquivos(), "x").unwrap_err().starts_with("J-F2: "));
        assert_eq!(mf(vec![], &arquivos(), &arquivos(), &hc).unwrap_err(), "R-2.1: execuções diferem dos critérios");
        let mut x = execucao(0, SAIDA_OK, None).como_objeto().unwrap().clone();
        x.inserir("extra", Valor::Nulo);
        assert!(mf(vec![Valor::Objeto(x)], &arquivos(), &arquivos(), &hc).unwrap_err().starts_with("J-M3: "));
        let mut x = execucao(0, SAIDA_OK, None).como_objeto().unwrap().clone();
        x.inserir("comando", j("[\"python\", \"t.py\"]"));
        assert_eq!(mf(vec![Valor::Objeto(x)], &arquivos(), &arquivos(), &hc).unwrap_err(), "R-2.1: instrumento gravado diverge");
        // controles presentes sem o critério declarar
        assert!(mf(vec![execucao(0, SAIDA_OK, Some(vec![]))], &arquivos(), &arquivos(), &hc).unwrap_err().starts_with("J-M3: "));
        // arquivo apagado durante a execução: fecho FAIL (não UNPROVEN)
        let m = mf(e(), &arquivos(), &inv(&[("t.py", H1)]), &hc).unwrap();
        assert_eq!(m.veredito, Veredito::Fail);
    }
}

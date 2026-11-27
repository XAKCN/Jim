//! §3 de contract/SEMANTICA.md: gate com a semântica de `gate43` da v5-alpha.
//!
//! Garantias (§3.6): `Parte::passou` é privado e só é chamado na linha 6 de
//! §3.2; o veredito PASS só nasce no passo 5 de §3.5. Não há PASS padrão.

use crate::json::{id_valido, lista_ids, padrao_data, repr_opt, repr_py, e_data, strip, Objeto, Valor};

pub const CAMPOS: [&str; 4] = ["adequado", "atual", "falha_critica", "conflito"];
pub const EXIGIDO: [bool; 4] = [true, true, false, false];
pub const EXTERNOS: [&str; 5] = ["execucao", "teste", "medicao", "fonte_externa", "revisao_independente"];
pub const INTERNOS: [&str; 2] = ["autocritica", "critico_isolado"];
pub const COM_DATA: [&str; 2] = ["fonte_externa", "medicao"];
pub const VPD: &str = "vinculada_dependencias";
pub const SEVERIDADES: [&str; 4] = ["critica", "alta", "media", "baixa"];
pub const CHAVES_REGISTRO: [&str; 8] =
    ["adequado", "atual", "falha_critica", "conflito", "instrumento", "dependencias", "valido_ate", "versao_contrato"];

/// Resultado de três valores do gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Res {
    Pass,
    Fail,
    Unknown,
}

impl Res {
    pub fn nome(self) -> &'static str {
        match self {
            Res::Pass => "PASS",
            Res::Fail => "FAIL",
            Res::Unknown => "UNKNOWN",
        }
    }
}

/// `{"resultado", "motivo"}`; motivo None se e só se PASS.
#[derive(Clone, Debug)]
pub struct Parte {
    pub resultado: Res,
    pub motivo: Option<String>,
}

impl Parte {
    /// Único construtor de PASS (§3.6). Privado ao módulo.
    fn passou() -> Parte {
        Parte { resultado: Res::Pass, motivo: None }
    }
    pub fn fail(m: String) -> Parte {
        Parte { resultado: Res::Fail, motivo: Some(m) }
    }
    pub fn unknown(m: String) -> Parte {
        Parte { resultado: Res::Unknown, motivo: Some(m) }
    }
    pub fn e_pass(&self) -> bool {
        self.resultado == Res::Pass
    }
    pub fn para_valor(&self) -> Valor {
        let mut o = Objeto::novo();
        o.inserir("resultado", Valor::texto(self.resultado.nome()));
        o.inserir("motivo", self.motivo.as_deref().map(Valor::texto).unwrap_or(Valor::Nulo));
        Valor::Objeto(o)
    }
}

fn unk(m: &str) -> Result<Parte, String> {
    Ok(Parte::unknown(m.to_string()))
}

/// Parâmetro `exige_humano`: omitido = Bool(false); não-bool guarda o repr.
#[derive(Clone, Debug)]
pub enum Humano {
    Bool(bool),
    Invalido(String),
}

/// Versão do contrato: `Sem` em `avaliar_criterio_43` (passos 1d e 2l não existem).
#[derive(Clone, Copy)]
pub enum Versao<'a> {
    Sem,
    Com(Option<&'a Valor>),
}

#[derive(Clone, Debug)]
pub struct SaidaGate {
    pub veredito: Res,
    pub contrato: Parte,
    /// (critério, parte, severidade) na ordem dos obrigatórios.
    pub criterios: Vec<(String, Parte, Option<String>)>,
    pub integracao: Parte,
    pub ordem_falhas: Vec<String>,
}

impl SaidaGate {
    pub fn para_valor(&self) -> Valor {
        let mut cr = Objeto::novo();
        for (k, p, s) in &self.criterios {
            let mut o = match p.para_valor() {
                Valor::Objeto(o) => o,
                _ => Objeto::novo(),
            };
            o.inserir("severidade", s.as_deref().map(Valor::texto).unwrap_or(Valor::Nulo));
            cr.inserir(k, Valor::Objeto(o));
        }
        let mut o = Objeto::novo();
        o.inserir("veredito", Valor::texto(self.veredito.nome()));
        o.inserir("contrato", self.contrato.para_valor());
        o.inserir("criterios", Valor::Objeto(cr));
        o.inserir("integracao", self.integracao.para_valor());
        o.inserir("ordem_falhas", Valor::Lista(self.ordem_falhas.iter().map(|s| Valor::texto(s)).collect()));
        Valor::Objeto(o)
    }

    /// Motivo de V11/P6: primeiro não nulo entre contrato, critério de
    /// `ordem_falhas[0]` e integração.
    pub fn primeiro_motivo(&self) -> String {
        if let Some(m) = &self.contrato.motivo {
            return m.clone();
        }
        if let Some(k) = self.ordem_falhas.first() {
            if let Some((_, p, _)) = self.criterios.iter().find(|(n, _, _)| n == k) {
                if let Some(m) = &p.motivo {
                    return m.clone();
                }
            }
        }
        if let Some(m) = &self.integracao.motivo {
            return m.clone();
        }
        "J-V11: gate não dá PASS".to_string()
    }
}

// ---------------------------------------------------------------- auxiliares

fn texto_em(l: Option<&Valor>, s: &str) -> bool {
    matches!(l, Some(Valor::Lista(v)) if v.iter().any(|x| x.e_texto(s)))
}

fn so_um(l: Option<&Valor>, s: &str) -> bool {
    matches!(l, Some(Valor::Lista(v)) if v.len() == 1 && v[0].e_texto(s))
}

fn minuscula_ascii(s: &str) -> String {
    strip(s).to_ascii_lowercase()
}

/// E1 e E2.
fn validar_parametros<'a>(hoje: &'a Valor, correntes: &Valor) -> Result<&'a str, String> {
    let h = match hoje {
        Valor::Texto(s) if e_data(s) => s.as_str(),
        _ => return Err(format!("R-2.14: hoje deve ser data AAAA-MM-DD, recebido {}", repr_py(hoje))),
    };
    if !matches!(correntes, Valor::Objeto(_)) {
        return Err(format!("R-2.21: correntes deve ser dict, recebido {}", correntes.nome_tipo()));
    }
    Ok(h)
}

/// E4c/E4d: `valido_ate` datado.
fn data_limite(v: &Valor) -> Result<&str, String> {
    match v {
        Valor::Texto(s) if padrao_data(s) => {
            if e_data(s) {
                Ok(s)
            } else {
                Err(format!("R-2.14: valido_ate não é uma data válida: {}", repr_py(v)))
            }
        }
        _ => Err(format!("R-2.14: valido_ate fora do formato AAAA-MM-DD: {}", repr_py(v))),
    }
}

/// E4.
fn validar_dominio(registro: Option<&Valor>) -> Result<(), String> {
    let r = match registro {
        Some(Valor::Objeto(r)) => r,
        _ => return Ok(()),
    };
    for campo in CAMPOS {
        if let Some(v) = r.bruto(campo) {
            if !matches!(v, Valor::Bool(_)) {
                return Err(format!("R-2.11: {campo} deve ser bool, recebido {}", repr_py(v)));
            }
        }
    }
    let i = r.get("instrumento");
    if id_valido(i) {
        let s = i.and_then(|v| v.como_texto()).unwrap_or("");
        if !EXTERNOS.contains(&s) && !INTERNOS.contains(&s) {
            return Err(format!("R-2.13: instrumento desconhecido: {}", repr_opt(i)));
        }
    }
    if let Some(v) = r.get("valido_ate") {
        if !v.e_texto(VPD) {
            data_limite(v)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- §3.2

/// §3.2 `avaliar`.
#[allow(clippy::too_many_arguments)]
pub fn avaliar(
    registro: Option<&Valor>,
    hoje: &Valor,
    aceitos: Option<&Valor>,
    correntes: &Valor,
    familias: Option<&Valor>,
    humano: &Humano,
    versao: Versao<'_>,
) -> Result<Parte, String> {
    let hoje = validar_parametros(hoje, correntes)?;
    let exige = match humano {
        Humano::Bool(b) => *b,
        Humano::Invalido(r) => return Err(format!("R-2.11: exige_humano deve ser bool, recebido {r}")),
    };
    validar_dominio(registro)?;
    let correntes = correntes.como_objeto().ok_or("J-0: falha interna: correntes")?;
    let familias = match familias {
        Some(Valor::Nulo) | None => None,
        f => f,
    };

    // 1
    if !lista_ids(aceitos) {
        return unk("R-2.8: critério sem lista de instrumentos aceitos");
    }
    if familias.is_some() && !lista_ids(familias) {
        return unk("R-2.19: lista de famílias de revisor aceitas malformada ou vazia");
    }
    if exige && !so_um(familias, "humano") {
        return unk("R-2.19: critério de aprovação humana sem familias_revisor_aceitas igual a [\"humano\"]");
    }
    if let Versao::Com(v) = versao {
        if !id_valido(v) {
            return unk("R-2.7: contrato sem versão");
        }
    }

    // 2
    let r = match registro {
        Some(Valor::Objeto(r)) => r,
        _ => return unk("R-2.9: registro de evidência ausente"),
    };
    for campo in CAMPOS {
        if !r.contem(campo) {
            return Ok(Parte::unknown(format!("R-2.9: campo {campo} omitido")));
        }
    }
    let deps = match r.get("dependencias") {
        Some(Valor::Objeto(d)) if !d.is_empty() => d,
        _ => return unk("R-2.12: registro sem dependências identificadas"),
    };
    for (k, v) in deps.iter() {
        if !(id_valido(Some(&Valor::texto(k))) && id_valido(Some(v))) {
            return Ok(Parte::unknown(format!("R-2.12: dependência sem identificador: {}", crate::json::repr_texto(k))));
        }
    }
    let i = match r.get("instrumento") {
        Some(Valor::Texto(s)) if !strip(s).is_empty() => s.as_str(),
        _ => return unk("R-2.9: campo instrumento omitido"),
    };
    if INTERNOS.contains(&i) {
        return Ok(Parte::fail(format!("R-2.13: instrumento interno não sustenta PASS: {i}")));
    }
    if !texto_em(aceitos, i) {
        return Ok(Parte::fail(format!("R-2.13: instrumento fora da lista aceita: {i}")));
    }
    if i == "revisao_independente" {
        let (fb, fr) = (r.get("familia_builder"), r.get("familia_revisor"));
        let (fb, fr) = match (fb, fr) {
            (Some(Valor::Texto(a)), Some(Valor::Texto(b))) if !strip(a).is_empty() && !strip(b).is_empty() => (a, b),
            _ => return unk("R-2.16: família do builder ou do revisor ausente"),
        };
        if !fb.is_ascii() || !fr.is_ascii() {
            return unk("J-G2: família fora do ASCII; comparação de caixa não é portável");
        }
        if minuscula_ascii(fb) == minuscula_ascii(fr) {
            return Ok(Parte::fail(format!("R-2.16: revisor da mesma família do builder: {fr}")));
        }
    }
    let valido_ate = match r.get("valido_ate") {
        None => return unk("R-2.14: registro sem valido_ate"),
        Some(v) => v,
    };
    if !valido_ate.e_texto(VPD) {
        let limite = data_limite(valido_ate)?;
        if hoje > limite {
            return Ok(Parte::fail(format!("R-2.14: evidência vencida em {limite}")));
        }
    }
    for (n, campo) in CAMPOS.iter().enumerate() {
        let v = matches!(r.bruto(campo), Some(Valor::Bool(true)));
        if !matches!(r.bruto(campo), Some(Valor::Bool(_))) || v != EXIGIDO[n] {
            let py = |b: bool| if b { "True" } else { "False" };
            return Ok(Parte::fail(format!("R-2.10: {campo} deve ser {}, registrado {}", py(EXIGIDO[n]), py(v))));
        }
    }
    if let Versao::Com(versao) = versao {
        let vc = match r.get("versao_contrato") {
            None => return unk("R-2.9: campo versao_contrato omitido"),
            Some(v) => v,
        };
        let igual = matches!((vc, versao), (Valor::Texto(a), Some(Valor::Texto(b))) if a == b);
        if !igual {
            return Ok(Parte::fail(format!(
                "R-2.5: evidência da versão {} do contrato, vigente {}",
                repr_py(vc),
                repr_opt(versao)
            )));
        }
    }

    // 3
    if COM_DATA.contains(&i) && valido_ate.e_texto(VPD) {
        return Ok(Parte::unknown(format!("R-2.23: {i} exige valido_ate datado, não {VPD}")));
    }

    // 4
    let mut chaves: Vec<&str> = deps.chaves().collect();
    chaves.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    for k in &chaves {
        if !id_valido(correntes.get(k)) {
            return Ok(Parte::unknown(format!(
                "R-2.21: dependência {} sem identificador corrente",
                crate::json::repr_texto(k)
            )));
        }
    }
    for k in &chaves {
        let (v, c) = (deps.get(k), correntes.get(k));
        let igual = matches!((v, c), (Some(Valor::Texto(a)), Some(Valor::Texto(b))) if a == b);
        if !igual {
            return Ok(Parte::fail(format!(
                "R-2.21: dependência {} mudou: registro {}, corrente {}",
                crate::json::repr_texto(k),
                repr_opt(v),
                repr_opt(c)
            )));
        }
    }

    // 5
    if familias.is_some() {
        let fr = r.get("familia_revisor");
        let fr = match fr {
            Some(Valor::Texto(s)) if !strip(s).is_empty() => s.as_str(),
            _ => return unk("R-2.19: registro sem familia_revisor"),
        };
        if !texto_em(familias, fr) {
            return Ok(Parte::fail(format!("R-2.19: família de revisor não aceita: {fr}")));
        }
    }
    if exige {
        if i != "revisao_independente" {
            return Ok(Parte::unknown(format!("R-2.19: aprovação humana exige revisao_independente, registrado {i}")));
        }
        let fr = r.get("familia_revisor");
        if !matches!(fr, Some(v) if v.e_texto("humano")) {
            return Ok(Parte::fail(format!("R-2.19: aprovação humana registrada pela família {}", repr_opt(fr))));
        }
    }

    // 6
    Ok(Parte::passou())
}

/// `avaliar_criterio_43`.
pub fn avaliar_criterio_43(
    registro: &Valor,
    hoje: &Valor,
    aceitos: &Valor,
    correntes: &Valor,
    familias: &Valor,
    humano: &Humano,
) -> Result<Parte, String> {
    avaliar(Some(registro), hoje, Some(aceitos), correntes, Some(familias), humano, Versao::Sem)
}

// ---------------------------------------------------------------- §3.3

fn avaliar_integracao(
    integracao: &Valor,
    hoje: &Valor,
    aceitos: Option<&Valor>,
    correntes: &Valor,
    candidato: Option<&Valor>,
    versao: Option<&Valor>,
) -> Result<Parte, String> {
    validar_parametros(hoje, correntes)?;
    validar_dominio(Some(integracao))?;
    if !lista_ids(aceitos) {
        return unk("R-2.22: contrato sem integracao.evidencias_aceitas");
    }
    let r = match integracao {
        Valor::Objeto(r) if CHAVES_REGISTRO.iter().any(|k| r.contem(k)) => r,
        _ => return unk("R-2.22: integração não é registro de evidência no formato 2.2"),
    };
    let cand = match candidato {
        Some(Valor::Texto(s)) if !strip(s).is_empty() => s,
        _ => return unk("R-2.22: contrato sem candidato"),
    };
    let reg = match r.get("dependencias") {
        Some(Valor::Objeto(d)) => d.get("candidato"),
        _ => None,
    };
    let reg = match reg {
        Some(Valor::Texto(s)) if !strip(s).is_empty() => s,
        _ => return unk("R-2.22: integração sem dependencias['candidato']"),
    };
    if reg != cand {
        return Ok(Parte::fail(format!(
            "R-2.22: integração de outro candidato: {}, contrato {}",
            crate::json::repr_texto(reg),
            crate::json::repr_texto(cand)
        )));
    }
    avaliar(Some(integracao), hoje, aceitos, correntes, None, &Humano::Bool(false), Versao::Com(versao))
}

// ---------------------------------------------------------------- §3.4

/// `_validar_contrato`: nunca levanta; só PASS ou UNKNOWN.
pub fn validar_contrato_gate(contrato: &Valor) -> Parte {
    let c = match contrato {
        Valor::Objeto(c) if id_valido(c.get("versao")) => c,
        _ => return Parte::unknown("R-2.7: contrato sem versão".into()),
    };
    if !id_valido(c.get("candidato")) {
        return Parte::unknown("R-2.7: contrato sem candidato".into());
    }
    let obrig = c.get("obrigatorios");
    if !lista_ids(obrig) {
        return Parte::unknown("R-2.8: lista de critérios obrigatórios vazia ou malformada".into());
    }
    let obrig: Vec<&str> = obrig.and_then(|v| v.como_lista()).map(|l| l.iter().filter_map(|x| x.como_texto()).collect()).unwrap_or_default();
    let aceitas = c.get("evidencias_aceitas").and_then(|v| v.como_objeto());
    for k in &obrig {
        if aceitas.is_none() || !lista_ids(aceitas.and_then(|a| a.get(k))) {
            return Parte::unknown(format!("R-2.8: critério {k} sem instrumentos aceitos"));
        }
    }
    let sev = c.get("severidade").and_then(|v| v.como_objeto());
    for k in &obrig {
        let s = sev.and_then(|o| o.get(k));
        if !matches!(s, Some(Valor::Texto(t)) if SEVERIDADES.contains(&t.as_str())) {
            return Parte::unknown(format!("R-2.24: critério {k} sem severidade válida: {}", repr_opt(s)));
        }
    }
    let vazio = Valor::Objeto(Objeto::novo());
    let familias = match c.bruto("familias_revisor_aceitas") {
        None => &vazio,
        Some(v) => v,
    };
    let familias = match familias {
        Valor::Objeto(f) => f,
        _ => return Parte::unknown("R-2.19: familias_revisor_aceitas deve ser dict por critério".into()),
    };
    for (k, l) in familias.iter() {
        if !lista_ids(Some(l)) {
            return Parte::unknown(format!(
                "R-2.19: famílias de revisor aceitas malformadas em {}",
                crate::json::repr_texto(k)
            ));
        }
    }
    let vazia = Valor::Lista(Vec::new());
    let humanos = match c.bruto("aprovacao_humana") {
        None => &vazia,
        Some(v) => v,
    };
    let humanos = match humanos {
        Valor::Lista(l) if l.iter().all(|x| id_valido(Some(x))) => l,
        _ => return Parte::unknown("R-2.19: aprovacao_humana malformada".into()),
    };
    for h in humanos {
        let k = h.como_texto().unwrap_or("");
        if !obrig.contains(&k) {
            return Parte::unknown(format!("R-2.19: critério de aprovação humana {k} fora dos obrigatórios"));
        }
        let a = aceitas.and_then(|a| a.get(k));
        if !so_um(a, "revisao_independente") {
            return Parte::unknown(format!(
                "R-2.18: critério {k} de aprovação humana com evidencias_aceitas diferente de [\"revisao_independente\"]: {}",
                repr_opt(a)
            ));
        }
        if !so_um(familias.get(k), "humano") {
            return Parte::unknown(format!(
                "R-2.19: critério {k} de aprovação humana sem familias_revisor_aceitas igual a [\"humano\"]"
            ));
        }
    }
    let integ = c.get("integracao").and_then(|v| v.como_objeto());
    if integ.is_none() || !lista_ids(integ.and_then(|i| i.get("evidencias_aceitas"))) {
        return Parte::unknown("R-2.22: contrato sem integracao.evidencias_aceitas".into());
    }
    Parte::passou()
}

// ---------------------------------------------------------------- §3.5

fn posto(s: &Option<String>) -> i64 {
    match s {
        Some(s) => SEVERIDADES.iter().position(|x| x == s).map(|p| p as i64).unwrap_or(-1),
        None => -1,
    }
}

/// §3.5 `gate_43`.
pub fn gate_43(
    contrato: &Valor,
    avaliacoes: &Valor,
    integracao: &Valor,
    hoje: &Valor,
    correntes: &Valor,
) -> Result<SaidaGate, String> {
    validar_parametros(hoje, correntes)?;
    let avaliacoes = match avaliacoes {
        Valor::Objeto(a) => a,
        _ => return Err("R-2.9: avaliacoes deve ser dict ou pares (criterio, registro)".into()),
    };
    let mut rc = validar_contrato_gate(contrato);
    let vazio = Objeto::novo();
    let c = contrato.como_objeto().unwrap_or(&vazio);
    let obrig: Vec<String> = match c.get("obrigatorios") {
        Some(Valor::Lista(l)) => l
            .iter()
            .filter(|x| id_valido(Some(x)))
            .filter_map(|x| x.como_texto().map(String::from))
            .collect(),
        _ => Vec::new(),
    };
    let campo_obj = |k: &str| c.get(k).and_then(|v| v.como_objeto()).unwrap_or(&vazio);
    let aceitas = campo_obj("evidencias_aceitas");
    let sev = campo_obj("severidade");
    let fam = campo_obj("familias_revisor_aceitas");
    let sem_humanos = Vec::new();
    let hum = c.get("aprovacao_humana").and_then(|v| v.como_lista()).unwrap_or(&sem_humanos);
    let versao = c.get("versao");
    let premissas = match c.bruto("premissas") {
        None => &vazio,
        Some(Valor::Objeto(p)) => p,
        Some(_) => {
            rc = Parte::unknown("R-2.17: premissas deve ser dict por critério".into());
            &vazio
        }
    };

    let mut criterios: Vec<(String, Parte, Option<String>)> = Vec::new();
    for k in &obrig {
        let registro = avaliacoes.get(k);
        let humano = Humano::Bool(hum.iter().any(|h| h.e_texto(k)));
        let mut r = avaliar(registro, hoje, aceitas.get(k), correntes, fam.get(k), &humano, Versao::Com(versao))?;
        if r.e_pass() && premissas.contem(k) {
            let ids = premissas.bruto(k);
            let validos = match ids {
                Some(Valor::Lista(l)) => l.iter().all(|p| id_valido(Some(p)) && !p.e_texto("candidato")),
                _ => false,
            };
            if !validos {
                r = Parte::unknown("R-2.17: identificadores de premissas inválidos".into());
            } else {
                let deps = registro.and_then(|v| v.como_objeto()).and_then(|o| o.get("dependencias")).and_then(|d| d.como_objeto());
                for p in ids.and_then(|v| v.como_lista()).into_iter().flatten() {
                    let p = p.como_texto().unwrap_or("");
                    if !deps.map(|d| d.contem(p)).unwrap_or(false) {
                        r = Parte::unknown(format!(
                            "R-2.17: premissa {} sem verificação vinculada",
                            crate::json::repr_texto(p)
                        ));
                        break;
                    }
                }
            }
        }
        let s = match sev.get(k) {
            Some(Valor::Texto(s)) if SEVERIDADES.contains(&s.as_str()) => Some(s.clone()),
            _ => None,
        };
        if let Some(e) = criterios.iter_mut().find(|(n, _, _)| n == k) {
            e.1 = r;
            e.2 = s;
        } else {
            criterios.push((k.clone(), r, s));
        }
    }

    let aceitos_int = c.get("integracao").and_then(|v| v.como_objeto()).and_then(|i| i.get("evidencias_aceitas"));
    let res_int = avaliar_integracao(integracao, hoje, aceitos_int, correntes, c.get("candidato"), versao)?;

    let todos_pass = criterios.iter().all(|(_, p, _)| p.e_pass()) && res_int.e_pass();
    let algum_fail =
        criterios.iter().any(|(_, p, _)| p.resultado == Res::Fail) || res_int.resultado == Res::Fail;
    let veredito = if !rc.e_pass() {
        Res::Unknown
    } else if !criterios.is_empty() && todos_pass {
        Res::Pass
    } else if algum_fail {
        Res::Fail
    } else {
        Res::Unknown
    };

    let mut ordem: Vec<(i64, String)> = criterios
        .iter()
        .filter(|(_, p, _)| !p.e_pass())
        .map(|(n, _, s)| (posto(s), n.clone()))
        .collect();
    ordem.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.as_bytes().cmp(b.1.as_bytes())));
    Ok(SaidaGate {
        veredito,
        contrato: rc,
        criterios,
        integracao: res_int,
        ordem_falhas: ordem.into_iter().map(|(_, n)| n).collect(),
    })
}

// ---------------------------------------------------------------- testes

#[cfg(test)]
pub(crate) mod testes {
    use super::*;
    use crate::json::json_estrito;

    pub fn j(s: &str) -> Valor {
        json_estrito(s.as_bytes()).unwrap_or_else(|e| panic!("{e}: {s}"))
    }

    const HOJE: &str = "\"2026-01-10\"";

    fn reg_ok() -> Valor {
        j(r#"{"adequado": true, "atual": true, "falha_critica": false, "conflito": false,
             "instrumento": "execucao", "dependencias": {"a.py": "sha256:1"},
             "valido_ate": "vinculada_dependencias", "versao_contrato": "v1"}"#)
    }

    fn com(base: Valor, k: &str, v: Valor) -> Valor {
        let mut o = base.como_objeto().unwrap().clone();
        o.inserir(k, v);
        Valor::Objeto(o)
    }

    fn sem(base: Valor, k: &str) -> Valor {
        let mut o = base.como_objeto().unwrap().clone();
        o.remover(k);
        Valor::Objeto(o)
    }

    fn correntes() -> Valor {
        j(r#"{"a.py": "sha256:1", "b.py": "sha256:2", "candidato": "sha256:c", "contrato": "sha256:k"}"#)
    }

    fn av(reg: &Valor) -> Result<Parte, String> {
        avaliar_criterio_43(reg, &j(HOJE), &j(r#"["execucao", "revisao_independente", "medicao"]"#), &correntes(), &Valor::Nulo, &Humano::Bool(false))
    }

    fn av_v(reg: &Valor, fam: &Valor, humano: bool) -> Result<Parte, String> {
        avaliar(Some(reg), &j(HOJE), Some(&j(r#"["execucao", "revisao_independente", "medicao"]"#)), &correntes(), Some(fam), &Humano::Bool(humano), Versao::Com(Some(&j("\"v1\""))))
    }

    /// Resultado e começo do motivo; e garante que não é PASS.
    fn nao_pass(r: Result<Parte, String>, res: Res, prefixo: &str) {
        let p = r.unwrap_or_else(|e| panic!("exceção inesperada: {e}"));
        assert_ne!(p.resultado, Res::Pass, "PASS indevido: esperado {prefixo}");
        assert_eq!(p.resultado, res, "{:?}", p.motivo);
        let m = p.motivo.unwrap();
        assert!(m.starts_with(prefixo), "{m} !~ {prefixo}");
    }

    fn exc(r: Result<Parte, String>, msg: &str) {
        match r {
            Err(e) => assert_eq!(e, msg),
            Ok(p) => panic!("esperava exceção {msg}, veio {:?}", p),
        }
    }

    #[test]
    fn linha_6_pass() {
        let p = av(&reg_ok()).unwrap();
        assert!(p.e_pass() && p.motivo.is_none());
        let p = av_v(&reg_ok(), &Valor::Nulo, false).unwrap();
        assert!(p.e_pass());
    }

    #[test]
    fn e1_a_e4() {
        let a = j(r#"["execucao"]"#);
        let h = Humano::Bool(false);
        exc(avaliar_criterio_43(&reg_ok(), &j("\"2026-1-01\""), &a, &correntes(), &Valor::Nulo, &h),
            "R-2.14: hoje deve ser data AAAA-MM-DD, recebido '2026-1-01'");
        exc(avaliar_criterio_43(&reg_ok(), &Valor::Int(5), &a, &correntes(), &Valor::Nulo, &h),
            "R-2.14: hoje deve ser data AAAA-MM-DD, recebido 5");
        exc(avaliar_criterio_43(&reg_ok(), &j(HOJE), &a, &j("[]"), &Valor::Nulo, &h),
            "R-2.21: correntes deve ser dict, recebido list");
        exc(avaliar_criterio_43(&reg_ok(), &j(HOJE), &a, &Valor::Nulo, &Valor::Nulo, &h),
            "R-2.21: correntes deve ser dict, recebido NoneType");
        // E1 > E2
        exc(avaliar_criterio_43(&reg_ok(), &Valor::Nulo, &a, &Valor::Nulo, &Valor::Nulo, &h),
            "R-2.14: hoje deve ser data AAAA-MM-DD, recebido None");
        // E3: exige_humano=None
        exc(avaliar_criterio_43(&reg_ok(), &j(HOJE), &a, &correntes(), &Valor::Nulo, &Humano::Invalido("None".into())),
            "R-2.11: exige_humano deve ser bool, recebido None");
        // E4a
        exc(av(&com(reg_ok(), "atual", Valor::Int(1))), "R-2.11: atual deve ser bool, recebido 1");
        exc(av(&com(reg_ok(), "adequado", Valor::Nulo)), "R-2.11: adequado deve ser bool, recebido None");
        // E4b, sem strip
        exc(av(&com(reg_ok(), "instrumento", j("\" teste \""))), "R-2.13: instrumento desconhecido: ' teste '");
        // E4c / E4d
        exc(av(&com(reg_ok(), "valido_ate", j("\"2026-1-01\""))), "R-2.14: valido_ate fora do formato AAAA-MM-DD: '2026-1-01'");
        exc(av(&com(reg_ok(), "valido_ate", Valor::Int(3))), "R-2.14: valido_ate fora do formato AAAA-MM-DD: 3");
        exc(av(&com(reg_ok(), "valido_ate", j("\"2026-02-30\""))), "R-2.14: valido_ate não é uma data válida: '2026-02-30'");
        exc(av(&com(reg_ok(), "valido_ate", j("\"0000-01-01\""))), "R-2.14: valido_ate não é uma data válida: '0000-01-01'");
    }

    #[test]
    fn regra_1() {
        let h = Humano::Bool(false);
        for a in ["[]", "null", "[\"\"]", "\"execucao\"", "[\"execucao\", 1]"] {
            nao_pass(avaliar_criterio_43(&reg_ok(), &j(HOJE), &j(a), &correntes(), &Valor::Nulo, &h), Res::Unknown, "R-2.8: ");
        }
        for f in ["[]", "[\" \"]", "\"humano\"", "{}"] {
            nao_pass(av_v(&reg_ok(), &j(f), false), Res::Unknown, "R-2.19: lista de famílias");
        }
        nao_pass(av_v(&reg_ok(), &Valor::Nulo, true), Res::Unknown, "R-2.19: critério de aprovação humana");
        nao_pass(av_v(&reg_ok(), &j("[\"Humano\"]"), true), Res::Unknown, "R-2.19: critério de aprovação humana");
        for v in [None, Some(j("\" \"")), Some(Valor::Int(1))] {
            let r = avaliar(Some(&reg_ok()), &j(HOJE), Some(&j("[\"execucao\"]")), &correntes(), None, &h, Versao::Com(v.as_ref()));
            nao_pass(r, Res::Unknown, "R-2.7: contrato sem versão");
        }
    }

    #[test]
    fn regra_2() {
        nao_pass(av(&Valor::Nulo), Res::Unknown, "R-2.9: registro de evidência ausente");
        nao_pass(av(&j("[]")), Res::Unknown, "R-2.9: registro de evidência ausente");
        nao_pass(av(&sem(reg_ok(), "falha_critica")), Res::Unknown, "R-2.9: campo falha_critica omitido");
        nao_pass(av(&sem(sem(reg_ok(), "conflito"), "atual")), Res::Unknown, "R-2.9: campo atual omitido");
        nao_pass(av(&com(reg_ok(), "dependencias", j("{}"))), Res::Unknown, "R-2.12: registro sem dependências");
        nao_pass(av(&com(reg_ok(), "dependencias", j("[]"))), Res::Unknown, "R-2.12: registro sem dependências");
        // ordem de inserção no R-2.12
        let p = av(&com(reg_ok(), "dependencias", j(r#"{"z": "", "a": 1}"#))).unwrap();
        assert_eq!(p.motivo.as_deref(), Some("R-2.12: dependência sem identificador: 'z'"));
        nao_pass(av(&com(reg_ok(), "dependencias", j(r#"{" ": "x"}"#))), Res::Unknown, "R-2.12: dependência sem identificador: ' '");
        nao_pass(av(&sem(reg_ok(), "instrumento")), Res::Unknown, "R-2.9: campo instrumento omitido");
        nao_pass(av(&com(reg_ok(), "instrumento", j("\"\""))), Res::Unknown, "R-2.9: campo instrumento omitido");
        nao_pass(av(&com(reg_ok(), "instrumento", j("\"autocritica\""))), Res::Fail, "R-2.13: instrumento interno não sustenta PASS: autocritica");
        nao_pass(av(&com(reg_ok(), "instrumento", j("\"teste\""))), Res::Fail, "R-2.13: instrumento fora da lista aceita: teste");
        // 2h
        let rev = com(reg_ok(), "instrumento", j("\"revisao_independente\""));
        nao_pass(av(&rev), Res::Unknown, "R-2.16: família do builder ou do revisor ausente");
        let rev2 = com(com(rev.clone(), "familia_builder", j("\"x\\u001c\"")), "familia_revisor", j("\"X\""));
        nao_pass(av(&rev2), Res::Fail, "R-2.16: revisor da mesma família do builder: X");
        let rev3 = com(com(rev.clone(), "familia_builder", j("\"é\"")), "familia_revisor", j("\"É\""));
        nao_pass(av(&rev3), Res::Unknown, "J-G2: ");
        let rev4 = com(com(rev.clone(), "familia_builder", j("\"a\"")), "familia_revisor", j("\"b\""));
        assert!(av(&rev4).unwrap().e_pass());
        // 2i / 2j
        nao_pass(av(&sem(reg_ok(), "valido_ate")), Res::Unknown, "R-2.14: registro sem valido_ate");
        nao_pass(av(&com(reg_ok(), "valido_ate", Valor::Nulo)), Res::Unknown, "R-2.14: registro sem valido_ate");
        let med = |d: &str| com(com(reg_ok(), "instrumento", j("\"medicao\"")), "valido_ate", Valor::texto(d));
        nao_pass(av(&med("2026-01-09")), Res::Fail, "R-2.14: evidência vencida em 2026-01-09");
        assert!(av(&med("2026-01-10")).unwrap().e_pass());
        assert!(av(&med("2026-01-11")).unwrap().e_pass());
        // 2k
        nao_pass(av(&com(reg_ok(), "adequado", Valor::Bool(false))), Res::Fail, "R-2.10: adequado deve ser True, registrado False");
        nao_pass(av(&com(reg_ok(), "conflito", Valor::Bool(true))), Res::Fail, "R-2.10: conflito deve ser False, registrado True");
        // 2l
        nao_pass(av_v(&sem(reg_ok(), "versao_contrato"), &Valor::Nulo, false), Res::Unknown, "R-2.9: campo versao_contrato omitido");
        nao_pass(av_v(&com(reg_ok(), "versao_contrato", Valor::Nulo), &Valor::Nulo, false), Res::Unknown, "R-2.9: campo versao_contrato omitido");
        nao_pass(av_v(&com(reg_ok(), "versao_contrato", Valor::Int(1)), &Valor::Nulo, false), Res::Fail, "R-2.5: evidência da versão 1 do contrato, vigente 'v1'");
        nao_pass(av_v(&com(reg_ok(), "versao_contrato", j("[\"v1\"]")), &Valor::Nulo, false), Res::Fail, "R-2.5: evidência da versão ['v1']");
        // avaliar_criterio_43 não confere versão
        assert!(av(&sem(reg_ok(), "versao_contrato")).unwrap().e_pass());
    }

    #[test]
    fn regras_3_a_5() {
        nao_pass(av(&com(reg_ok(), "instrumento", j("\"medicao\""))), Res::Unknown, "R-2.23: medicao exige valido_ate datado, não vinculada_dependencias");
        // 4a: duas ausentes, relata a menor em ordem de código
        let deps = j(r#"{"z.py": "x", "B.py": "y", "a.py": "sha256:1"}"#);
        nao_pass(av(&com(reg_ok(), "dependencias", deps)), Res::Unknown, "R-2.21: dependência 'B.py' sem identificador corrente");
        // 4a antes de 4b
        let deps = j(r#"{"a.py": "sha256:outro", "zz": "q"}"#);
        nao_pass(av(&com(reg_ok(), "dependencias", deps)), Res::Unknown, "R-2.21: dependência 'zz' sem identificador corrente");
        let deps = j(r#"{"b.py": "sha256:9", "a.py": "sha256:8"}"#);
        nao_pass(av(&com(reg_ok(), "dependencias", deps)), Res::Fail, "R-2.21: dependência 'a.py' mudou: registro 'sha256:8', corrente 'sha256:1'");
        // 5a / 5b
        nao_pass(av_v(&reg_ok(), &j("[\"humano\"]"), false), Res::Unknown, "R-2.19: registro sem familia_revisor");
        nao_pass(av_v(&com(reg_ok(), "familia_revisor", j("\"Humano\"")), &j("[\"humano\"]"), false), Res::Fail, "R-2.19: família de revisor não aceita: Humano");
        assert!(av_v(&com(reg_ok(), "familia_revisor", j("\"humano\"")), &j("[\"humano\"]"), false).unwrap().e_pass());
        // 5c / 5d
        nao_pass(av_v(&com(reg_ok(), "familia_revisor", j("\"humano\"")), &j("[\"humano\"]"), true), Res::Unknown, "R-2.19: aprovação humana exige revisao_independente, registrado execucao");
        let rev = com(com(com(reg_ok(), "instrumento", j("\"revisao_independente\"")), "familia_builder", j("\"m\"")), "familia_revisor", j("\"humano\""));
        assert!(av_v(&rev, &j("[\"humano\"]"), true).unwrap().e_pass());
    }

    // ------------------------------------------------ gate_43

    pub fn contrato_ok() -> Valor {
        j(r#"{"versao": "v1", "candidato": "sha256:c", "obrigatorios": ["t1", "t2"],
             "evidencias_aceitas": {"t1": ["execucao"], "t2": ["execucao"]},
             "severidade": {"t1": "alta", "t2": "critica"},
             "integracao": {"evidencias_aceitas": ["execucao"]}}"#)
    }

    fn integ_ok() -> Valor {
        j(r#"{"adequado": true, "atual": true, "falha_critica": false, "conflito": false,
             "instrumento": "execucao", "dependencias": {"candidato": "sha256:c"},
             "valido_ate": "vinculada_dependencias", "versao_contrato": "v1"}"#)
    }

    fn avs() -> Valor {
        objeto_de(vec![("t1", reg_ok()), ("t2", reg_ok())])
    }

    fn objeto_de(p: Vec<(&str, Valor)>) -> Valor {
        crate::json::objeto(p)
    }

    fn g(c: &Valor, a: &Valor, i: &Valor) -> Result<SaidaGate, String> {
        gate_43(c, a, i, &j(HOJE), &correntes())
    }

    fn g_nao_pass(c: &Valor, a: &Valor, i: &Valor, ver: Res) -> SaidaGate {
        let s = g(c, a, i).unwrap_or_else(|e| panic!("exceção: {e}"));
        assert_ne!(s.veredito, Res::Pass, "PASS indevido");
        assert_eq!(s.veredito, ver);
        s
    }

    #[test]
    fn gate_pass_e_formato() {
        let s = g(&contrato_ok(), &avs(), &integ_ok()).unwrap();
        assert_eq!(s.veredito, Res::Pass);
        let v = s.para_valor();
        assert_eq!(
            String::from_utf8(crate::json::json_canonico(&v)).unwrap(),
            r#"{"contrato":{"motivo":null,"resultado":"PASS"},"criterios":{"t1":{"motivo":null,"resultado":"PASS","severidade":"alta"},"t2":{"motivo":null,"resultado":"PASS","severidade":"critica"}},"integracao":{"motivo":null,"resultado":"PASS"},"ordem_falhas":[],"veredito":"PASS"}"#
        );
        let chaves: Vec<&str> = v.como_objeto().unwrap().chaves().collect();
        assert_eq!(chaves, ["veredito", "contrato", "criterios", "integracao", "ordem_falhas"]);
    }

    #[test]
    fn contrato_c1_a_c10() {
        let casos: Vec<(Valor, &str)> = vec![
            (Valor::Nulo, "R-2.7: contrato sem versão"),
            (sem(contrato_ok(), "versao"), "R-2.7: contrato sem versão"),
            (com(contrato_ok(), "candidato", j("\" \"")), "R-2.7: contrato sem candidato"),
            (com(contrato_ok(), "obrigatorios", j("[]")), "R-2.8: lista de critérios obrigatórios"),
            (com(contrato_ok(), "obrigatorios", j("[\"t1\", 2]")), "R-2.8: lista de critérios obrigatórios"),
            (com(contrato_ok(), "evidencias_aceitas", j(r#"{"t1": ["execucao"]}"#)), "R-2.8: critério t2 sem instrumentos aceitos"),
            (com(contrato_ok(), "severidade", j(r#"{"t1": "alta", "t2": "grave"}"#)), "R-2.24: critério t2 sem severidade válida: 'grave'"),
            (com(contrato_ok(), "severidade", Valor::Nulo), "R-2.24: critério t1 sem severidade válida: None"),
            (com(contrato_ok(), "familias_revisor_aceitas", Valor::Nulo), "R-2.19: familias_revisor_aceitas deve ser dict"),
            (com(contrato_ok(), "familias_revisor_aceitas", j(r#"{"x": []}"#)), "R-2.19: famílias de revisor aceitas malformadas em 'x'"),
            (com(contrato_ok(), "aprovacao_humana", Valor::Nulo), "R-2.19: aprovacao_humana malformada"),
            (com(contrato_ok(), "aprovacao_humana", j("[\"t9\"]")), "R-2.19: critério de aprovação humana t9 fora dos obrigatórios"),
            (com(contrato_ok(), "aprovacao_humana", j("[\"t1\"]")), "R-2.18: critério t1 de aprovação humana com evidencias_aceitas diferente de [\"revisao_independente\"]: ['execucao']"),
            (com(com(contrato_ok(), "aprovacao_humana", j("[\"t1\"]")), "evidencias_aceitas", j(r#"{"t1": ["revisao_independente"], "t2": ["execucao"]}"#)),
             "R-2.19: critério t1 de aprovação humana sem familias_revisor_aceitas igual a [\"humano\"]"),
            (sem(contrato_ok(), "integracao"), "R-2.22: contrato sem integracao.evidencias_aceitas"),
            (com(contrato_ok(), "integracao", j(r#"{"evidencias_aceitas": []}"#)), "R-2.22: contrato sem integracao.evidencias_aceitas"),
        ];
        for (c, m) in casos {
            let p = validar_contrato_gate(&c);
            assert_eq!(p.resultado, Res::Unknown, "{m}");
            assert!(p.motivo.as_deref().unwrap().starts_with(m), "{:?} !~ {m}", p.motivo);
            // o gate nunca dá PASS com contrato UNKNOWN
            let s = g(&c, &avs(), &integ_ok()).unwrap();
            assert_eq!(s.veredito, Res::Unknown, "{m}");
        }
        assert!(validar_contrato_gate(&contrato_ok()).e_pass());
    }

    #[test]
    fn integracao_i1_a_i6() {
        let s = g_nao_pass(&com(contrato_ok(), "integracao", j("{}")), &avs(), &integ_ok(), Res::Unknown);
        assert!(s.integracao.motivo.as_deref().unwrap().starts_with("R-2.22: contrato sem integracao"));
        // integração 4.2 e true
        for i in [j(r#"{"ok": true, "versao_entregue": "sha256:c"}"#), Valor::Bool(true), Valor::Nulo] {
            let s = g_nao_pass(&contrato_ok(), &avs(), &i, Res::Unknown);
            assert_eq!(s.integracao.motivo.as_deref(), Some("R-2.22: integração não é registro de evidência no formato 2.2"));
        }
        // I3: candidato só é conferido aqui quando o contrato... (rc já dá UNKNOWN)
        let s = g_nao_pass(&sem(contrato_ok(), "candidato"), &avs(), &integ_ok(), Res::Unknown);
        assert_eq!(s.integracao.motivo.as_deref(), Some("R-2.22: contrato sem candidato"));
        let s = g_nao_pass(&contrato_ok(), &avs(), &com(integ_ok(), "dependencias", j("{}")), Res::Unknown);
        assert_eq!(s.integracao.motivo.as_deref(), Some("R-2.22: integração sem dependencias['candidato']"));
        let s = g_nao_pass(&contrato_ok(), &avs(), &com(integ_ok(), "dependencias", j(r#"{"candidato": "sha256:x"}"#)), Res::Fail);
        assert_eq!(s.integracao.motivo.as_deref(), Some("R-2.22: integração de outro candidato: 'sha256:x', contrato 'sha256:c'"));
        // I6: candidato ausente de correntes
        let mut cor = correntes().como_objeto().unwrap().clone();
        cor.remover("candidato");
        let s = gate_43(&contrato_ok(), &avs(), &integ_ok(), &j(HOJE), &Valor::Objeto(cor)).unwrap();
        assert_eq!(s.veredito, Res::Unknown);
        assert_eq!(s.integracao.motivo.as_deref(), Some("R-2.21: dependência 'candidato' sem identificador corrente"));
        // I6 com versão divergente
        let s = g_nao_pass(&contrato_ok(), &avs(), &com(integ_ok(), "versao_contrato", j("\"v0\"")), Res::Fail);
        assert!(s.integracao.motivo.as_deref().unwrap().starts_with("R-2.5: "));
    }

    #[test]
    fn precedencia_de_excecoes() {
        let a_ruim = objeto_de(vec![("t1", com(reg_ok(), "atual", Valor::Int(1))), ("t2", com(reg_ok(), "conflito", Valor::Int(2)))]);
        let i_ruim = com(integ_ok(), "adequado", j("\"s\""));
        // E1 > E2
        assert!(gate_43(&contrato_ok(), &a_ruim, &i_ruim, &Valor::Nulo, &Valor::Nulo).unwrap_err().starts_with("R-2.14: "));
        assert!(gate_43(&contrato_ok(), &a_ruim, &i_ruim, &j(HOJE), &Valor::Nulo).unwrap_err().starts_with("R-2.21: "));
        // E4 do 1º obrigatório > 2º > integração
        assert_eq!(g(&contrato_ok(), &a_ruim, &i_ruim).unwrap_err(), "R-2.11: atual deve ser bool, recebido 1");
        let a2 = objeto_de(vec![("t1", reg_ok()), ("t2", com(reg_ok(), "conflito", Valor::Int(2)))]);
        assert_eq!(g(&contrato_ok(), &a2, &i_ruim).unwrap_err(), "R-2.11: conflito deve ser bool, recebido 2");
        assert_eq!(g(&contrato_ok(), &avs(), &i_ruim).unwrap_err(), "R-2.11: adequado deve ser bool, recebido 's'");
        // avaliacoes em pares (J-G3)
        assert_eq!(g(&contrato_ok(), &j(r#"[["t1", {}]]"#), &integ_ok()).unwrap_err(), "R-2.9: avaliacoes deve ser dict ou pares (criterio, registro)");
        // registro fora dos obrigatórios com adequado: 1 não levanta
        let a3 = objeto_de(vec![("t1", reg_ok()), ("t2", reg_ok()), ("fora", com(reg_ok(), "adequado", Valor::Int(1)))]);
        assert_eq!(g(&contrato_ok(), &a3, &integ_ok()).unwrap().veredito, Res::Pass);
    }

    #[test]
    fn premissas() {
        let s = g_nao_pass(&com(contrato_ok(), "premissas", Valor::Nulo), &avs(), &integ_ok(), Res::Unknown);
        assert_eq!(s.contrato.motivo.as_deref(), Some("R-2.17: premissas deve ser dict por critério"));
        // substitui o motivo anterior do contrato
        let c = com(sem(contrato_ok(), "integracao"), "premissas", j("[]"));
        let s = g_nao_pass(&c, &avs(), &integ_ok(), Res::Unknown);
        assert_eq!(s.contrato.motivo.as_deref(), Some("R-2.17: premissas deve ser dict por critério"));
        let s = g_nao_pass(&com(contrato_ok(), "premissas", j(r#"{"t1": ["candidato"]}"#)), &avs(), &integ_ok(), Res::Unknown);
        assert_eq!(s.criterios[0].1.motivo.as_deref(), Some("R-2.17: identificadores de premissas inválidos"));
        let s = g_nao_pass(&com(contrato_ok(), "premissas", j(r#"{"t1": "a.py"}"#)), &avs(), &integ_ok(), Res::Unknown);
        assert_eq!(s.criterios[0].1.motivo.as_deref(), Some("R-2.17: identificadores de premissas inválidos"));
        let s = g_nao_pass(&com(contrato_ok(), "premissas", j(r#"{"t2": ["a.py", "q.py"]}"#)), &avs(), &integ_ok(), Res::Unknown);
        assert_eq!(s.criterios[1].1.motivo.as_deref(), Some("R-2.17: premissa 'q.py' sem verificação vinculada"));
        let s = g(&com(contrato_ok(), "premissas", j(r#"{"t1": [], "t2": ["a.py"]}"#)), &avs(), &integ_ok()).unwrap();
        assert_eq!(s.veredito, Res::Pass);
    }

    #[test]
    fn obrigatorios_e_ordem_falhas() {
        // repetidos e id inválido (rc UNKNOWN, mas avalia os válidos e mantém a 1ª posição)
        let c = com(contrato_ok(), "obrigatorios", j(r#"["t2", "", "t1", "t2"]"#));
        let s = g_nao_pass(&c, &avs(), &integ_ok(), Res::Unknown);
        let nomes: Vec<&str> = s.criterios.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(nomes, ["t2", "t1"]);
        // ordem_falhas: severidade inválida primeiro; empate por nome
        let c = j(r#"{"versao": "v1", "candidato": "sha256:c", "obrigatorios": ["b", "a", "c", "d"],
             "evidencias_aceitas": {"a": ["execucao"], "b": ["execucao"], "c": ["execucao"], "d": ["execucao"]},
             "severidade": {"a": "baixa", "b": "baixa", "c": "nada", "d": "critica"},
             "integracao": {"evidencias_aceitas": ["execucao"]}}"#);
        let s = g_nao_pass(&c, &j("{}"), &integ_ok(), Res::Unknown);
        assert_eq!(s.ordem_falhas, ["c", "d", "a", "b"]);
        assert_eq!(s.criterios[2].2, None);
        // veredito: FAIL se alguma parte FAIL, com contrato PASS
        let a = objeto_de(vec![("t1", com(reg_ok(), "adequado", Valor::Bool(false)))]);
        let s = g_nao_pass(&contrato_ok(), &a, &integ_ok(), Res::Fail);
        assert_eq!(s.ordem_falhas, ["t2", "t1"]);
        assert_eq!(s.primeiro_motivo(), "R-2.9: registro de evidência ausente");
        // exige_humano via aprovacao_humana
        let c = j(r#"{"versao": "v1", "candidato": "sha256:c", "obrigatorios": ["h"],
             "evidencias_aceitas": {"h": ["revisao_independente"]}, "severidade": {"h": "alta"},
             "familias_revisor_aceitas": {"h": ["humano"]}, "aprovacao_humana": ["h"],
             "integracao": {"evidencias_aceitas": ["execucao"]}}"#);
        let rev = com(com(com(reg_ok(), "instrumento", j("\"revisao_independente\"")), "familia_builder", j("\"m\"")), "familia_revisor", j("\"humano\""));
        let s = g(&c, &objeto_de(vec![("h", rev.clone())]), &integ_ok()).unwrap();
        assert_eq!(s.veredito, Res::Pass);
        let s = g_nao_pass(&c, &objeto_de(vec![("h", com(rev, "familia_revisor", j("\"Humano\"")))]), &integ_ok(), Res::Fail);
        assert!(s.criterios[0].1.motivo.as_deref().unwrap().starts_with("R-2.19: família de revisor não aceita"));
    }

    #[test]
    fn nunca_pass_sem_criterio_ou_com_parte_ruim() {
        // tudo PASS menos uma parte → nunca PASS
        for reg in [com(reg_ok(), "atual", Valor::Bool(false)), sem(reg_ok(), "adequado"), Valor::Nulo] {
            let a = objeto_de(vec![("t1", reg_ok()), ("t2", reg)]);
            assert_ne!(g(&contrato_ok(), &a, &integ_ok()).unwrap().veredito, Res::Pass);
        }
        assert_ne!(g(&contrato_ok(), &avs(), &com(integ_ok(), "atual", Valor::Bool(false))).unwrap().veredito, Res::Pass);
    }
}

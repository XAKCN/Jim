//! §4 de contract/SEMANTICA.md: contrato `jim-contrato/1` e derivação de
//! registros a partir do observado (semântica de `tools/release.py`).

use crate::gate::SEVERIDADES;
use crate::hash::{arvore, sha256_hex, validar_caminho, validar_inventario};
use crate::json::{e_data, e_espaco, e_hex64, id_valido, iguais, json_estrito, repr_texto, strip, Objeto, Valor};
use std::collections::{HashMap, HashSet};

pub const ESQUEMA_CONTRATO: &str = "jim-contrato/1";
const CHAVES_CONTRATO: [&str; 6] = ["esquema", "versao", "data_referencia", "arquivos", "criterios", "premissas"];
const CHAVES_CRITERIO: [&str; 9] =
    ["comando", "aceitos", "severidade", "reprova", "esperado", "regra", "depende_de", "timeout_s", "controles_negativos"];

pub type Inventario = Vec<(String, String)>;

// ---------------------------------------------------------------- §4.1

/// Lista de caminhos resolvidos: §2.5 e sem repetição.
pub fn validar_lista_caminhos(arquivos: &[String]) -> Result<(), String> {
    let mut vistos = HashSet::new();
    for a in arquivos {
        validar_caminho(a)?;
        if !vistos.insert(a.as_str()) {
            return Err(format!("J-C3: caminho repetido: {}", repr_texto(a)));
        }
    }
    Ok(())
}

/// §4.1 `validar_contrato`.
pub fn validar_contrato(c: &Valor, arquivos: &[String]) -> Result<(), String> {
    validar_lista_caminhos(arquivos)?;
    validar_contrato_com(c, Some(arquivos))
}

/// §4.1; com `arquivos` None, pula só as regras que dependem da lista de
/// arquivos (uso de `contrato_gate`, que não a recebe).
pub fn validar_contrato_com(c: &Valor, arquivos: Option<&[String]>) -> Result<(), String> {
    let o = c.como_objeto().ok_or("J-K1: contrato deve ser objeto")?;
    if !matches!(o.bruto("esquema"), Some(v) if v.e_texto(ESQUEMA_CONTRATO)) {
        return Err("J-K2: esquema deve ser jim-contrato/1".into());
    }
    for k in o.chaves() {
        if !CHAVES_CONTRATO.contains(&k) {
            return Err(format!("J-K5: chave desconhecida: {}", repr_texto(k)));
        }
    }
    if !id_valido(o.get("versao")) {
        return Err("R-2.1/R-2.5: identidade do contrato inválida".into());
    }
    if !matches!(o.get("data_referencia"), Some(Valor::Texto(s)) if e_data(s)) {
        return Err("R-2.14: data_referencia fora do formato AAAA-MM-DD".into());
    }
    let padroes_ok = matches!(o.get("arquivos"), Some(Valor::Lista(l))
        if !l.is_empty() && l.iter().all(|x| matches!(x, Valor::Texto(s) if !s.is_empty())));
    if !padroes_ok {
        return Err("J-K3: arquivos deve ser lista não vazia de padrões".into());
    }
    let criterios = match o.get("criterios") {
        Some(Valor::Objeto(cr)) if !cr.is_empty() && !cr.contem("integracao") => cr,
        _ => return Err("R-2.8: critérios ausentes ou reservados".into()),
    };
    let em_arquivos = |p: &Valor| match (arquivos, p) {
        (None, _) => true,
        (Some(a), Valor::Texto(s)) => a.iter().any(|x| x == s),
        _ => false,
    };
    for (nome, crit) in criterios.iter() {
        let crit = match crit {
            Valor::Objeto(cr) if !strip(nome).is_empty() => cr,
            _ => return Err("R-2.8: critério malformado".into()),
        };
        for k in crit.chaves() {
            if !CHAVES_CRITERIO.contains(&k) {
                return Err(format!("J-K5: chave desconhecida: {}", repr_texto(k)));
            }
        }
        if !matches!(crit.bruto("severidade"), Some(Valor::Texto(s)) if SEVERIDADES.contains(&s.as_str())) {
            return Err(format!("R-2.24: severidade obrigatória: {nome}"));
        }
        let cmd = match crit.bruto("comando") {
            Some(Valor::Lista(l)) if !l.is_empty() && l.iter().all(|x| matches!(x, Valor::Texto(s) if !s.is_empty())) => l,
            _ => return Err(format!("R-2.1: comando inválido: {nome}")),
        };
        if !em_arquivos(&cmd[0]) || !cmd[0].como_texto().unwrap_or("").ends_with(".py") {
            return Err("R-3.13: instrumento Python fora da cópia".into());
        }
        if !id_valido(crit.get("reprova")) {
            return Err("R-2.1: condição de reprovação ausente".into());
        }
        // depende_de: padrão = arquivos
        let deps: Option<Vec<Valor>> = match crit.bruto("depende_de") {
            None => arquivos.map(|a| a.iter().map(|s| Valor::texto(s)).collect()),
            Some(Valor::Lista(l)) if !l.is_empty() && l.iter().all(em_arquivos) => Some(l.clone()),
            Some(_) => return Err("R-2.12: dependências fora do candidato".into()),
        };
        match crit.bruto("timeout_s") {
            None => {}
            Some(Valor::Int(t)) if *t > 0 => {}
            Some(_) => return Err("R-5.3: timeout inválido".into()),
        }
        if !matches!(crit.bruto("aceitos"), Some(Valor::Lista(l)) if !l.is_empty()) {
            return Err("R-2.8: instrumentos aceitos ausentes".into());
        }
        if !crit.contem("esperado") && !matches!(crit.bruto("regra"), Some(v) if v.e_texto("todos_aprovados")) {
            return Err("R-2.1: regra inválida".into());
        }
        if let Some(controles) = crit.bruto("controles_negativos") {
            let ok = match controles {
                Valor::Lista(l) if !l.is_empty() => l.iter().all(|ct| match ct {
                    Valor::Objeto(m) if !m.is_empty() => m.iter().all(|(p, t)| {
                        let em_deps = match &deps {
                            None => true,
                            Some(d) => d.iter().any(|x| x.e_texto(p)),
                        };
                        em_deps && !p.ends_with(".py") && matches!(t, Valor::Nulo | Valor::Texto(_))
                    }),
                    _ => false,
                }),
                _ => false,
            };
            if !ok {
                return Err(format!("R-3.4: controles negativos exigem alterações de artefatos do critério: {nome}"));
            }
        }
    }
    Ok(())
}

fn criterios_de(c: &Valor) -> Result<&Objeto, String> {
    c.como_objeto()
        .and_then(|o| o.get("criterios"))
        .and_then(|v| v.como_objeto())
        .ok_or_else(|| "R-2.8: critérios ausentes ou reservados".to_string())
}

// ---------------------------------------------------------------- §4.2

/// §4.2 `contrato_gate`.
pub fn contrato_gate(c: &Valor, candidato: &str) -> Result<Valor, String> {
    validar_contrato_com(c, None)?;
    let o = c.como_objeto().ok_or("J-K1: contrato deve ser objeto")?;
    let cr = criterios_de(c)?;
    let mut g = Objeto::novo();
    g.inserir("versao", o.bruto("versao").cloned().unwrap_or(Valor::Nulo));
    g.inserir("candidato", Valor::texto(candidato));
    g.inserir("obrigatorios", Valor::Lista(cr.chaves().map(Valor::texto).collect()));
    let campo = |k: &str| {
        let mut m = Objeto::novo();
        for (n, v) in cr.iter() {
            m.inserir(n, v.como_objeto().and_then(|x| x.bruto(k)).cloned().unwrap_or(Valor::Nulo));
        }
        Valor::Objeto(m)
    };
    g.inserir("evidencias_aceitas", campo("aceitos"));
    g.inserir("severidade", campo("severidade"));
    g.inserir("integracao", crate::json::objeto(vec![("evidencias_aceitas", Valor::Lista(vec![Valor::texto("execucao")]))]));
    if let Some(p) = o.bruto("premissas") {
        g.inserir("premissas", p.clone());
    }
    Ok(Valor::Objeto(g))
}

// ---------------------------------------------------------------- §4.3

fn e_separador_linha(c: u32) -> bool {
    matches!(c, 0x0A | 0x0B | 0x0C | 0x0D | 0x1C | 0x1D | 0x1E | 0x85 | 0x2028 | 0x2029)
}

fn strip_pontos(s: &[u32]) -> &[u32] {
    let a = s.iter().position(|c| !e_espaco(*c)).unwrap_or(s.len());
    let z = s.iter().rposition(|c| !e_espaco(*c)).map(|p| p + 1).unwrap_or(a);
    &s[a..z.max(a)]
}

/// §4.3 sobre pontos de código (substitutos isolados permitidos, vindos de
/// `str` do Python): última linha JSON-objeto ou None. Nunca falha.
pub fn ultima_linha_json_pontos(texto: &[u32]) -> Option<Valor> {
    // 1. J-S1
    let mut norm: Vec<u32> = Vec::with_capacity(texto.len());
    let mut i = 0;
    while i < texto.len() {
        if texto[i] == 0x0D {
            norm.push(0x0A);
            if i + 1 < texto.len() && texto[i + 1] == 0x0A {
                i += 1;
            }
        } else {
            norm.push(texto[i]);
        }
        i += 1;
    }
    // 2.
    let t = strip_pontos(&norm);
    // 3. splitlines
    let mut linhas: Vec<&[u32]> = Vec::new();
    let mut a = 0;
    let mut j = 0;
    while j < t.len() {
        if e_separador_linha(t[j]) {
            linhas.push(&t[a..j]);
            if t[j] == 0x0D && j + 1 < t.len() && t[j + 1] == 0x0A {
                j += 1;
            }
            a = j + 1;
        }
        j += 1;
    }
    if a < t.len() {
        linhas.push(&t[a..]);
    }
    // 4.
    for linha in linhas.iter().rev() {
        if strip_pontos(linha).first() == Some(&('{' as u32)) {
            let mut s = String::with_capacity(linha.len());
            for c in linha.iter() {
                // substituto isolado na linha: o parse da v5-alpha recusaria → None
                s.push(char::from_u32(*c)?);
            }
            return json_estrito(s.as_bytes()).ok();
        }
    }
    None
}

/// §4.3 para texto Rust.
pub fn ultima_linha_json(texto: &str) -> Option<Valor> {
    let p: Vec<u32> = texto.chars().map(|c| c as u32).collect();
    ultima_linha_json_pontos(&p)
}

// ---------------------------------------------------------------- §4.4

/// §4.4 `avaliar_saida`.
pub fn avaliar_saida(saida: &Valor, crit: &Valor) -> Result<bool, String> {
    let s = match saida {
        Valor::Objeto(s) => s,
        _ => return Ok(false),
    };
    let crit = crit.como_objeto().ok_or("R-2.1: critério sem esperado/regra reconhecida")?;
    if let Some(e) = crit.bruto("esperado") {
        return Ok(iguais(saida, e));
    }
    if matches!(crit.bruto("regra"), Some(v) if v.e_texto("todos_aprovados")) {
        let casos = s.get("casos").and_then(|v| v.como_int());
        let aprov = s.get("aprovados").and_then(|v| v.como_int());
        let ok = matches!(casos, Some(c) if c > 0)
            && aprov.is_some()
            && aprov == casos
            && matches!(s.get("falhas"), Some(Valor::Lista(l)) if l.is_empty())
            && zero_ou_ausente(s, "erros_enumeracao");
        return Ok(ok);
    }
    Err("R-2.1: critério sem esperado/regra reconhecida".into())
}

/// `type(d.get(k, 0)) is int and d.get(k, 0) == 0`.
fn zero_ou_ausente(s: &Objeto, k: &str) -> bool {
    matches!(s.bruto(k), None | Some(Valor::Int(0)))
}

// ---------------------------------------------------------------- §4.5

fn mesmo_tipo(a: &Valor, b: &Valor) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// `_formato_esperado`.
pub fn formato(s: &Valor, e: &Valor) -> bool {
    if !mesmo_tipo(s, e) {
        return false;
    }
    match (s, e) {
        (Valor::Objeto(so), Valor::Objeto(eo)) => {
            let ks: HashSet<&str> = so.chaves().collect();
            let ke: HashSet<&str> = eo.chaves().collect();
            ks == ke && eo.iter().all(|(k, v)| so.bruto(k).map(|x| formato(x, v)).unwrap_or(false))
        }
        (Valor::Lista(sl), Valor::Lista(el)) if !el.is_empty() => {
            sl.iter().all(|item| el.iter().any(|m| formato(item, m)))
        }
        _ => true,
    }
}

fn razao_de_falha(f: &Valor) -> bool {
    let f = match f {
        Valor::Objeto(o) => {
            if o.contem("caso") {
                o.bruto("caso")
            } else {
                o.bruto("bateria")
            }
        }
        v => Some(v),
    };
    id_valido(f)
}

/// `_rejeicao_observada`.
fn rejeicao_observada(saida: Option<&Valor>, crit: &Objeto, crit_v: &Valor) -> bool {
    let s = match saida {
        Some(Valor::Objeto(s)) if !s.is_empty() && !s.contem("erro") => s,
        _ => return false,
    };
    let erros_ok = match s.bruto("erros") {
        None | Some(Valor::Int(0)) => true,
        Some(Valor::Lista(l)) => l.is_empty(),
        Some(Valor::Objeto(o)) => o.is_empty(),
        _ => false,
    };
    if !erros_ok {
        return false;
    }
    if matches!(s.get("estado"), Some(v) if v.e_texto("UNKNOWN")) {
        return false;
    }
    if let Some(e) = crit.bruto("esperado") {
        let saida_v = saida.unwrap_or(&Valor::Nulo);
        if !formato(saida_v, e) {
            return false;
        }
        let casos_ok = match s.bruto("casos") {
            None => true,
            Some(Valor::Int(c)) => *c > 0,
            _ => false,
        };
        if !casos_ok || !zero_ou_ausente(s, "erros_enumeracao") {
            return false;
        }
        return matches!(avaliar_saida(saida_v, crit_v), Ok(false));
    }
    if matches!(crit.bruto("regra"), Some(v) if v.e_texto("todos_aprovados")) {
        let casos = s.get("casos").and_then(|v| v.como_int());
        let aprov = s.get("aprovados").and_then(|v| v.como_int());
        let (casos, aprov) = match (casos, aprov) {
            (Some(c), Some(a)) if c > 0 && 0 <= a && a < c => (c, a),
            _ => return false,
        };
        let falhas_ok = matches!(s.get("falhas"), Some(Valor::Lista(l))
            if l.len() as i64 == casos - aprov && l.iter().all(razao_de_falha));
        return falhas_ok && zero_ou_ausente(s, "erros_enumeracao");
    }
    false
}

fn tamanho(v: &Valor) -> Option<usize> {
    match v {
        Valor::Lista(l) => Some(l.len()),
        Valor::Objeto(o) => Some(o.len()),
        Valor::Texto(s) => Some(s.chars().count()),
        _ => None,
    }
}

/// §4.5 `controles_aprovados` (J-S3: `stderr == ""`, presente).
pub fn controles_aprovados(ex: &Valor, crit: &Valor) -> bool {
    let (ex, cr) = match (ex, crit) {
        (Valor::Objeto(e), Valor::Objeto(c)) => (e, c),
        _ => return false,
    };
    let n = match cr.bruto("controles_negativos").and_then(tamanho) {
        Some(n) if n > 0 => n,
        _ => return false,
    };
    let obtidos = match ex.get("controles_negativos") {
        Some(Valor::Lista(l)) if l.len() == n => l,
        _ => return false,
    };
    obtidos.iter().all(|c| match c {
        Valor::Objeto(c) => {
            matches!(c.bruto("alterado"), Some(Valor::Bool(true)))
                && matches!(c.bruto("exit_code"), Some(Valor::Int(0 | 1)))
                && matches!(c.bruto("stderr"), Some(v) if v.e_texto(""))
                && rejeicao_observada(c.get("saida"), cr, crit)
        }
        _ => false,
    })
}

// ---------------------------------------------------------------- §4.6

/// Saída de `derivar_registros`.
#[derive(Clone, Debug)]
pub struct Registros {
    pub avaliacoes: Valor,
    pub integracao: Valor,
    pub correntes: Valor,
}

impl Registros {
    pub fn para_valor(&self) -> Valor {
        crate::json::objeto(vec![
            ("avaliacoes", self.avaliacoes.clone()),
            ("integracao", self.integracao.clone()),
            ("correntes", self.correntes.clone()),
        ])
    }
}

fn stream_confere(o: &Objeto) -> bool {
    let (out, err) = match (o.bruto("stdout"), o.bruto("stderr")) {
        (Some(Valor::Texto(a)), Some(Valor::Texto(b))) => (a, b),
        _ => return false,
    };
    let hash_ok = |k: &str, s: &str| matches!(o.bruto(k), Some(Valor::Texto(h)) if *h == sha256_hex(s.as_bytes()));
    let saida = match o.bruto("saida") {
        Some(v) => v,
        None => return false,
    };
    hash_ok("stdout_sha256", out)
        && hash_ok("stderr_sha256", err)
        && iguais(saida, &ultima_linha_json(out).unwrap_or(Valor::Nulo))
}

/// Pré-condições de §4.6 (V8): critérios, comandos, streams (J-N7) e formato gravado.
pub fn conferir_execucoes(c: &Valor, execucoes: &Valor) -> Result<(), String> {
    let cr = criterios_de(c)?;
    let diferem = || "R-2.1: execuções diferem dos critérios".to_string();
    let exs = execucoes.como_lista().ok_or_else(diferem)?;
    if exs.len() != cr.len() {
        return Err(diferem());
    }
    for (ex, nome) in exs.iter().zip(cr.chaves()) {
        let ok = matches!(ex.como_objeto().and_then(|o| o.bruto("criterio")), Some(v) if v.e_texto(nome));
        if !ok {
            return Err(diferem());
        }
    }
    for (ex, (_, crit)) in exs.iter().zip(cr.iter()) {
        let o = ex.como_objeto().ok_or_else(diferem)?;
        let mut esperado = vec![Valor::texto("python")];
        if let Some(Valor::Lista(l)) = crit.como_objeto().and_then(|x| x.bruto("comando")) {
            esperado.extend(l.iter().cloned());
        }
        let cmd_ok = matches!(o.bruto("comando"), Some(v) if iguais(v, &Valor::Lista(esperado)));
        if !cmd_ok || !matches!(o.bruto("exit_code"), Some(Valor::Int(_))) {
            return Err("R-2.1: instrumento gravado diverge".into());
        }
    }
    for (ex, nome) in exs.iter().zip(cr.chaves()) {
        let o = ex.como_objeto().ok_or_else(diferem)?;
        let mut ok = stream_confere(o);
        if let Some(cts) = o.bruto("controles_negativos") {
            ok = ok
                && match cts {
                    Valor::Lista(l) => l.iter().all(|c| c.como_objeto().map(stream_confere).unwrap_or(false)),
                    _ => false,
                };
        }
        if !ok {
            return Err(format!("J-N7: stream gravado não confere: {nome}"));
        }
        if !crate::fecho::execucao_ok(ex, true) {
            return Err(format!("R-2.1: execução gravada malformada: {nome}"));
        }
    }
    Ok(())
}

fn hex_ou_erro(nome: &str, h: &str) -> Result<(), String> {
    if e_hex64(h) {
        Ok(())
    } else {
        Err(format!("J-F2: hash inválido em {}", repr_texto(nome)))
    }
}

/// §4.6 `derivar_registros`.
pub fn derivar_registros(
    c: &Valor,
    execucoes: &Valor,
    executados: &Inventario,
    atuais: &Inventario,
    hc: &str,
    hc_depois: &str,
) -> Result<Registros, String> {
    let chaves: Vec<String> = executados.iter().map(|(p, _)| p.clone()).collect();
    validar_contrato(c, &chaves)?;
    validar_inventario(executados)?;
    validar_inventario(atuais)?;
    hex_ou_erro("contrato_sha256", hc)?;
    hex_ou_erro("contrato_sha256_depois", hc_depois)?;
    conferir_execucoes(c, execucoes)?;

    let o = c.como_objeto().ok_or("J-K1: contrato deve ser objeto")?;
    let versao = o.bruto("versao").cloned().unwrap_or(Valor::Nulo);
    let cr = criterios_de(c)?;
    let exs = execucoes.como_lista().ok_or("R-2.1: execuções diferem dos critérios")?;

    let candidato = format!("sha256:{}", arvore(executados));
    let mut correntes = Objeto::novo();
    for (p, h) in atuais {
        correntes.inserir(p, Valor::Texto(format!("sha256:{h}")));
    }
    correntes.inserir("candidato", Valor::Texto(format!("sha256:{}", arvore(atuais))));
    correntes.inserir("contrato", Valor::Texto(format!("sha256:{hc_depois}")));
    let mapa: HashMap<&str, &str> = correntes.iter().filter_map(|(k, v)| v.como_texto().map(|s| (k, s))).collect();
    let exec_mapa: HashMap<&str, &str> = executados.iter().map(|(p, h)| (p.as_str(), h.as_str())).collect();

    let mut avaliacoes = Objeto::novo();
    for ((nome, crit), ex) in cr.iter().zip(exs.iter()) {
        let cro = crit.como_objeto().ok_or("R-2.8: critério malformado")?;
        let exo = ex.como_objeto().ok_or("R-2.1: execuções diferem dos critérios")?;
        let mut deps = Objeto::novo();
        let lista_deps: Vec<String> = match cro.bruto("depende_de") {
            Some(Valor::Lista(l)) => l.iter().filter_map(|x| x.como_texto().map(String::from)).collect(),
            _ => chaves.clone(),
        };
        for p in &lista_deps {
            let h = exec_mapa.get(p.as_str()).ok_or("R-2.12: dependências fora do candidato")?;
            deps.inserir(p, Valor::Texto(format!("sha256:{h}")));
        }
        deps.inserir("candidato", Valor::texto(&candidato));
        deps.inserir("contrato", Valor::Texto(format!("sha256:{hc}")));
        let exit = exo.bruto("exit_code").and_then(|v| v.como_int()).ok_or("R-2.1: instrumento gravado diverge")?;
        let saida = exo.bruto("saida").cloned().unwrap_or(Valor::Nulo);
        let adequado = exit == 0 && avaliar_saida(&saida, crit)?;
        let atual = deps.iter().all(|(p, h)| matches!((mapa.get(p), h), (Some(a), Valor::Texto(b)) if a == b));
        let falha_critica = exit != 0 || matches!(saida, Valor::Nulo);
        let mut r = Objeto::novo();
        r.inserir("adequado", Valor::Bool(adequado));
        r.inserir("atual", Valor::Bool(atual));
        r.inserir("falha_critica", Valor::Bool(falha_critica));
        r.inserir("conflito", Valor::Bool(false));
        r.inserir("instrumento", Valor::texto("execucao"));
        r.inserir("dependencias", Valor::Objeto(deps));
        r.inserir("versao_contrato", versao.clone());
        r.inserir("valido_ate", Valor::texto(crate::gate::VPD));
        let exige_controle = matches!(cro.bruto("severidade"), Some(v) if v.e_texto("critica")) || cro.contem("controles_negativos");
        if exige_controle && adequado && !controles_aprovados(ex, crit) {
            r.remover("adequado");
        }
        avaliacoes.inserir(nome, Valor::Objeto(r));
    }

    let iguais_inv = |a: &Inventario, b: &Inventario| {
        let ma: HashMap<&str, &str> = a.iter().map(|(p, h)| (p.as_str(), h.as_str())).collect();
        let mb: HashMap<&str, &str> = b.iter().map(|(p, h)| (p.as_str(), h.as_str())).collect();
        ma == mb
    };
    let confere = iguais_inv(executados, atuais) && hc == hc_depois;
    let mut integ = Objeto::novo();
    integ.inserir("adequado", Valor::Bool(confere));
    integ.inserir("atual", Valor::Bool(confere));
    integ.inserir("falha_critica", Valor::Bool(false));
    integ.inserir("conflito", Valor::Bool(false));
    integ.inserir("instrumento", Valor::texto("execucao"));
    integ.inserir(
        "dependencias",
        crate::json::objeto(vec![("candidato", Valor::texto(&candidato)), ("contrato", Valor::Texto(format!("sha256:{hc}")))]),
    );
    integ.inserir("versao_contrato", versao);
    integ.inserir("valido_ate", Valor::texto(crate::gate::VPD));
    Ok(Registros { avaliacoes: Valor::Objeto(avaliacoes), integracao: Valor::Objeto(integ), correntes: Valor::Objeto(correntes) })
}

// ---------------------------------------------------------------- testes

#[cfg(test)]
pub(crate) mod testes {
    use super::*;
    use crate::gate::{gate_43, Res};
    use crate::gate::testes::j;

    pub const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    pub const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    pub const HC: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    pub fn contrato(crit_extra: &str) -> Valor {
        j(&format!(
            r#"{{"esquema": "jim-contrato/1", "versao": "v1", "data_referencia": "2026-01-10",
                "arquivos": ["**/*.py", "dados.txt"],
                "criterios": {{"t": {{"comando": ["t.py"], "aceitos": ["execucao"], "severidade": "alta",
                                      "reprova": "falha", "esperado": {{"ok": true}}{crit_extra}}}}}}}"#
        ))
    }

    pub fn inv(pares: &[(&str, &str)]) -> Inventario {
        pares.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    pub fn arquivos() -> Inventario {
        inv(&[("t.py", H1), ("dados.txt", H2)])
    }

    pub fn ctl(stdout: &str, stderr: &str, alterado: bool, exit: i64) -> Valor {
        crate::json::objeto(vec![
            ("alterado", Valor::Bool(alterado)),
            ("exit_code", Valor::Int(exit)),
            ("stdout", Valor::texto(stdout)),
            ("stderr", Valor::texto(stderr)),
            ("saida", ultima_linha_json(stdout).unwrap_or(Valor::Nulo)),
            ("stdout_sha256", Valor::Texto(sha256_hex(stdout.as_bytes()))),
            ("stderr_sha256", Valor::Texto(sha256_hex(stderr.as_bytes()))),
        ])
    }

    /// Execução no formato gravado (§5.1).
    pub fn gravada(criterio: &str, cmd: &[&str], exit: i64, stdout: &str, controles: Option<Vec<Valor>>) -> Valor {
        let mut c = vec![Valor::texto("python")];
        c.extend(cmd.iter().map(|s| Valor::texto(s)));
        let mut o = Objeto::novo();
        o.inserir("criterio", Valor::texto(criterio));
        o.inserir("comando", Valor::Lista(c));
        o.inserir("exit_code", Valor::Int(exit));
        o.inserir("stdout", Valor::texto(stdout));
        o.inserir("stderr", Valor::texto(""));
        o.inserir("duracao_s", Valor::Float(0.5));
        if let Some(cs) = controles {
            o.inserir("controles_negativos", Valor::Lista(cs));
        }
        o.inserir("saida", ultima_linha_json(stdout).unwrap_or(Valor::Nulo));
        o.inserir("stdout_sha256", Valor::Texto(sha256_hex(stdout.as_bytes())));
        o.inserir("stderr_sha256", Valor::Texto(sha256_hex(b"")));
        Valor::Objeto(o)
    }

    /// derivar + contrato_gate + gate_43 (o que montar_fecho faz).
    fn veredito(c: &Valor, exs: Vec<Valor>, atuais: &Inventario, hc_depois: &str) -> (Res, crate::gate::SaidaGate, Registros) {
        let r = derivar_registros(c, &Valor::Lista(exs), &arquivos(), atuais, HC, hc_depois).unwrap();
        let cand = crate::hash::hash_fecho(&arquivos()).unwrap();
        let cg = contrato_gate(c, &cand).unwrap();
        let hoje = c.como_objeto().unwrap().get("data_referencia").unwrap().clone();
        let g = gate_43(&cg, &r.avaliacoes, &r.integracao, &hoje, &r.correntes).unwrap();
        (g.veredito, g, r)
    }

    fn nao_pass(c: &Valor, exs: Vec<Valor>, atuais: &Inventario, hc_depois: &str, res: Res, motivo: &str) {
        let (v, g, _) = veredito(c, exs, atuais, hc_depois);
        assert_ne!(v, Res::Pass, "PASS indevido: {motivo}");
        assert_eq!(v, res, "{:?}", g);
        let m = g.primeiro_motivo();
        assert!(m.starts_with(motivo), "{m} !~ {motivo}");
    }

    #[test]
    fn d_pass_com_esperado() {
        let (v, _, r) = veredito(&contrato(""), vec![gravada("t", &["t.py"], 0, "log\n{\"ok\": true}\n", None)], &arquivos(), HC);
        assert_eq!(v, Res::Pass);
        let av = r.avaliacoes.como_objeto().unwrap().get("t").unwrap().como_objeto().unwrap();
        let ch: Vec<&str> = av.chaves().collect();
        assert_eq!(ch, ["adequado", "atual", "falha_critica", "conflito", "instrumento", "dependencias", "versao_contrato", "valido_ate"]);
        let deps: Vec<&str> = av.get("dependencias").unwrap().como_objeto().unwrap().chaves().collect();
        assert_eq!(deps, ["t.py", "dados.txt", "candidato", "contrato"]);
        let cor: Vec<&str> = r.correntes.como_objeto().unwrap().chaves().collect();
        assert_eq!(cor, ["t.py", "dados.txt", "candidato", "contrato"]);
    }

    #[test]
    fn d_todos_aprovados() {
        let c = contrato(", \"regra\": \"todos_aprovados\"").como_objeto().unwrap().clone();
        let mut c = c;
        let mut cr = c.get("criterios").unwrap().como_objeto().unwrap().clone();
        let mut t = cr.get("t").unwrap().como_objeto().unwrap().clone();
        t.remover("esperado");
        cr.inserir("t", Valor::Objeto(t));
        c.inserir("criterios", Valor::Objeto(cr));
        let c = Valor::Objeto(c);
        let (v, _, _) = veredito(&c, vec![gravada("t", &["t.py"], 0, "{\"casos\": 3, \"aprovados\": 3, \"falhas\": []}", None)], &arquivos(), HC);
        assert_eq!(v, Res::Pass);
        for s in [
            "{\"casos\": 3, \"aprovados\": 2, \"falhas\": [\"x\"]}",
            "{\"casos\": 0, \"aprovados\": 0, \"falhas\": []}",
            "{\"casos\": true, \"aprovados\": true, \"falhas\": []}",
            "{\"casos\": 3, \"aprovados\": 3, \"falhas\": [], \"erros_enumeracao\": 1}",
            "{\"casos\": 3, \"aprovados\": 3, \"falhas\": [], \"erros_enumeracao\": null}",
            "{\"casos\": 3, \"aprovados\": 3}",
        ] {
            nao_pass(&c, vec![gravada("t", &["t.py"], 0, s, None)], &arquivos(), HC, Res::Fail, "R-2.10: adequado");
        }
    }

    #[test]
    fn d_critica_sem_controles_e_controles() {
        let c = contrato("").como_objeto().unwrap().clone();
        let mut c2 = c.clone();
        let mut cr = c2.get("criterios").unwrap().como_objeto().unwrap().clone();
        let mut t = cr.get("t").unwrap().como_objeto().unwrap().clone();
        t.inserir("severidade", Valor::texto("critica"));
        cr.inserir("t", Valor::Objeto(t.clone()));
        c2.inserir("criterios", Valor::Objeto(cr.clone()));
        let critica = Valor::Objeto(c2.clone());
        let ok = "{\"ok\": true}";
        nao_pass(&critica, vec![gravada("t", &["t.py"], 0, ok, None)], &arquivos(), HC, Res::Unknown, "R-2.9: campo adequado omitido");

        // com controles: rejeição observada válida → PASS
        t.inserir("controles_negativos", j(r#"[{"dados.txt": "lixo"}]"#));
        cr.inserir("t", Valor::Objeto(t));
        c2.inserir("criterios", Valor::Objeto(cr));
        let cc = Valor::Objeto(c2);
        let bom = ctl("{\"ok\": false}", "", true, 1);
        let (v, _, _) = veredito(&cc, vec![gravada("t", &["t.py"], 0, ok, Some(vec![bom.clone()]))], &arquivos(), HC);
        assert_eq!(v, Res::Pass);
        // controle com stderr
        let ruins = vec![
            ctl("{\"ok\": false}", "Traceback", true, 1),
            ctl("{\"ok\": false, \"erro\": \"x\"}", "", true, 1),
            ctl("{\"ok\": 0}", "", true, 1),             // formato bool × int
            ctl("{\"ok\": true}", "", true, 0),          // não rejeitou
            ctl("{\"ok\": false}", "", false, 1),        // não alterou
            ctl("{\"ok\": false}", "", true, 2),         // exit fora de {0, 1}
            ctl("nada", "", true, 1),                    // saída None
            ctl("{\"ok\": false, \"estado\": \"UNKNOWN\"}", "", true, 1),
        ];
        for r in ruins {
            nao_pass(&cc, vec![gravada("t", &["t.py"], 0, ok, Some(vec![r.clone()]))], &arquivos(), HC, Res::Unknown, "R-2.9: campo adequado omitido");
        }
        // número de controles diferente
        nao_pass(&cc, vec![gravada("t", &["t.py"], 0, ok, Some(vec![bom.clone(), bom]))], &arquivos(), HC, Res::Unknown, "R-2.9");
    }

    #[test]
    fn d_exit_saida_arvore_contrato() {
        let c = contrato("");
        let ok = "{\"ok\": true}";
        nao_pass(&c, vec![gravada("t", &["t.py"], 1, ok, None)], &arquivos(), HC, Res::Fail, "R-2.10: adequado deve ser True");
        nao_pass(&c, vec![gravada("t", &["t.py"], 0, "sem json", None)], &arquivos(), HC, Res::Fail, "R-2.10: adequado deve ser True");
        nao_pass(&c, vec![gravada("t", &["t.py"], 0, "{\"ok\": false}", None)], &arquivos(), HC, Res::Fail, "R-2.10: adequado");
        // arquivo mudado na execução
        let mudado = inv(&[("t.py", H1), ("dados.txt", H1)]);
        nao_pass(&c, vec![gravada("t", &["t.py"], 0, ok, None)], &mudado, HC, Res::Fail, "R-2.10: atual deve ser True");
        // arquivo apagado
        let apagado = inv(&[("t.py", H1)]);
        nao_pass(&c, vec![gravada("t", &["t.py"], 0, ok, None)], &apagado, HC, Res::Fail, "R-2.10: atual");
        // contrato mudado
        nao_pass(&c, vec![gravada("t", &["t.py"], 0, ok, None)], &arquivos(), H2, Res::Fail, "R-2.10: atual");
        let (_, g, _) = veredito(&c, vec![gravada("t", &["t.py"], 0, ok, None)], &arquivos(), H2);
        assert_eq!(g.integracao.resultado, Res::Fail);
    }

    #[test]
    fn d_precondicoes_e_n7() {
        let c = contrato("");
        let ok = "{\"ok\": true}";
        let d = |exs: Vec<Valor>| derivar_registros(&c, &Valor::Lista(exs), &arquivos(), &arquivos(), HC, HC).unwrap_err();
        assert_eq!(d(vec![]), "R-2.1: execuções diferem dos critérios");
        assert_eq!(d(vec![gravada("u", &["t.py"], 0, ok, None)]), "R-2.1: execuções diferem dos critérios");
        assert_eq!(d(vec![gravada("t", &["outro.py"], 0, ok, None)]), "R-2.1: instrumento gravado diverge");
        let mut adult = gravada("t", &["t.py"], 0, ok, None).como_objeto().unwrap().clone();
        adult.inserir("stdout", Valor::texto("{\"ok\": false}"));
        assert_eq!(d(vec![Valor::Objeto(adult.clone())]), "J-N7: stream gravado não confere: t");
        let mut s2 = gravada("t", &["t.py"], 0, ok, None).como_objeto().unwrap().clone();
        s2.inserir("saida", j("{\"ok\": false}"));
        assert_eq!(d(vec![Valor::Objeto(s2)]), "J-N7: stream gravado não confere: t");
        let mut s3 = gravada("t", &["t.py"], 0, ok, None).como_objeto().unwrap().clone();
        s3.remover("stderr");
        assert_eq!(d(vec![Valor::Objeto(s3)]), "J-N7: stream gravado não confere: t");
        let mut s4 = gravada("t", &["t.py"], 0, ok, None).como_objeto().unwrap().clone();
        s4.inserir("exit_code", Valor::Bool(false));
        assert_eq!(d(vec![Valor::Objeto(s4)]), "R-2.1: instrumento gravado diverge");
        let mut cc = ctl("{}", "", true, 1).como_objeto().unwrap().clone();
        cc.inserir("stderr", Valor::texto("x"));
        assert_eq!(d(vec![gravada("t", &["t.py"], 0, ok, Some(vec![Valor::Objeto(cc)]))]), "J-N7: stream gravado não confere: t");
    }

    #[test]
    fn derivacao_recusa_execucao_gravada_malformada() {
        let c = contrato("");
        let derivar = |e| derivar_registros(&c, &Valor::Lista(vec![e]), &arquivos(), &arquivos(), HC, HC);
        let valida = gravada("t", &["t.py"], 0, r#"{"ok":true}"#, None);
        assert!(derivar(valida.clone()).is_ok());
        for duracao in [None, Some(Valor::Nulo), Some(Valor::Bool(false)), Some(Valor::texto("0.5"))] {
            let mut e = valida.como_objeto().unwrap().clone();
            match duracao {
                None => e.remover("duracao_s"),
                Some(v) => e.inserir("duracao_s", v),
            }
            assert_eq!(derivar(Valor::Objeto(e)).unwrap_err(), "R-2.1: execução gravada malformada: t");
        }
        let mut controle = ctl(r#"{"ok":false}"#, "", true, 1).como_objeto().unwrap().clone();
        controle.inserir("alterado", Valor::Int(1));
        let e = gravada("t", &["t.py"], 0, r#"{"ok":true}"#, Some(vec![Valor::Objeto(controle)]));
        assert_eq!(derivar(e).unwrap_err(), "R-2.1: execução gravada malformada: t");
    }

    #[test]
    fn ultima_linha() {
        assert!(iguais(&ultima_linha_json("a\n{\"x\": 1}\nlixo depois").unwrap(), &j("{\"x\": 1}")));
        assert!(iguais(&ultima_linha_json("{\"x\": 1}\r\n{\"x\": 2}\r\n").unwrap(), &j("{\"x\": 2}")));
        assert!(iguais(&ultima_linha_json("{\"x\": 1}\r{\"x\": 2}").unwrap(), &j("{\"x\": 2}")));
        assert!(iguais(&ultima_linha_json("{\"x\": 1}\u{1c}{\"x\": 2}").unwrap(), &j("{\"x\": 2}")));
        assert!(iguais(&ultima_linha_json("{\"x\": 1}\u{2028}  {\"x\": 3}  ").unwrap(), &j("{\"x\": 3}")));
        // "\xa0{" : strip acha '{', mas o parse da linha recusa → None
        assert!(ultima_linha_json("{\"x\": 1}\n\u{a0}{\"x\": 2}").is_none());
        // última '{' inválida não volta para trás
        assert!(ultima_linha_json("{\"x\": 1}\n{inválido").is_none());
        assert!(ultima_linha_json("").is_none());
        assert!(ultima_linha_json("[1]\n2").is_none());
        // J-S2: inteiro grande e profundidade viram None
        assert!(ultima_linha_json("{\"x\": 9007199254740992}").is_none());
        assert!(ultima_linha_json("{\"x\": 1e400}").is_none());
        // substituto isolado na linha escolhida → None
        let p: Vec<u32> = "{\"x\": \"".chars().map(|c| c as u32).chain([0xD800]).chain("\"}".chars().map(|c| c as u32)).collect();
        assert!(ultima_linha_json_pontos(&p).is_none());
        let p2: Vec<u32> = [0xD800u32, 0x0A].into_iter().chain("{\"x\": 1}".chars().map(|c| c as u32)).collect();
        assert!(ultima_linha_json_pontos(&p2).is_some());
    }

    #[test]
    fn formato_e_saida() {
        assert!(!formato(&j("{\"a\": 1}"), &j("{\"a\": true}")));
        assert!(!formato(&j("{\"a\": 1}"), &j("{\"a\": 1.0}")));
        assert!(formato(&j("{\"a\": [1, 2]}"), &j("{\"a\": [3]}")));
        assert!(!formato(&j("{\"a\": [1, \"x\"]}"), &j("{\"a\": [3]}")));
        assert!(formato(&j("{\"a\": [1, \"x\"]}"), &j("{\"a\": []}")));
        assert!(!formato(&j("{\"a\": 1, \"b\": 1}"), &j("{\"a\": 1}")));
        assert!(!avaliar_saida(&j("[1]"), &j("{\"esperado\": [1]}")).unwrap());
        assert!(avaliar_saida(&j("{\"a\": 1}"), &j("{\"esperado\": {\"a\": 1}}")).unwrap());
        assert!(!avaliar_saida(&j("{\"a\": 1}"), &j("{\"esperado\": {\"a\": 1.0}}")).unwrap());
        assert_eq!(avaliar_saida(&j("{}"), &j("{\"regra\": \"sem_sobreviventes\"}")).unwrap_err(), "R-2.1: critério sem esperado/regra reconhecida");
        assert!(!controles_aprovados(&j("{}"), &j("{\"controles_negativos\": []}")));
    }

    #[test]
    fn k_validar_contrato() {
        let arqs: Vec<String> = vec!["t.py".into(), "dados.txt".into()];
        let v = |s: &str| validar_contrato(&j(s), &arqs);
        let base = |crit: &str| {
            format!(r#"{{"esquema": "jim-contrato/1", "versao": "v1", "data_referencia": "2026-01-10", "arquivos": ["*"], "criterios": {{"t": {crit}}}}}"#)
        };
        let ok = r#"{"comando": ["t.py"], "aceitos": ["execucao"], "severidade": "alta", "reprova": "x", "esperado": 1}"#;
        assert!(v(&base(ok)).is_ok());
        assert_eq!(v("[]").unwrap_err(), "J-K1: contrato deve ser objeto");
        assert_eq!(v("{\"esquema\": \"gauntlet\"}").unwrap_err(), "J-K2: esquema deve ser jim-contrato/1");
        let extra = base(ok).replacen("{", "{\"release\": \"0.0\", ", 1);
        assert_eq!(v(&extra).unwrap_err(), "J-K5: chave desconhecida: 'release'");
        assert_eq!(v(&base(ok).replace("\"v1\"", "\" \"")).unwrap_err(), "R-2.1/R-2.5: identidade do contrato inválida");
        assert_eq!(v(&base(ok).replace("2026-01-10", "2026-1-10")).unwrap_err(), "R-2.14: data_referencia fora do formato AAAA-MM-DD");
        assert_eq!(v(&base(ok).replace("[\"*\"]", "[\"\"]")).unwrap_err(), "J-K3: arquivos deve ser lista não vazia de padrões");
        assert_eq!(v(&base(ok).replace("\"t\":", "\"integracao\":")).unwrap_err(), "R-2.8: critérios ausentes ou reservados");
        assert_eq!(v(&base(ok).replace("\"t\":", "\" \":")).unwrap_err(), "R-2.8: critério malformado");
        assert_eq!(v(&base(&ok.replace("\"esperado\"", "\"esperad\""))).unwrap_err(), "J-K5: chave desconhecida: 'esperad'");
        assert_eq!(v(&base(&ok.replace("\"alta\"", "\"grave\""))).unwrap_err(), "R-2.24: severidade obrigatória: t");
        assert_eq!(v(&base(&ok.replace("[\"t.py\"]", "[]"))).unwrap_err(), "R-2.1: comando inválido: t");
        assert_eq!(v(&base(&ok.replace("[\"t.py\"]", "[\"u.py\"]"))).unwrap_err(), "R-3.13: instrumento Python fora da cópia");
        assert_eq!(v(&base(&ok.replace("[\"t.py\"]", "[\"dados.txt\"]"))).unwrap_err(), "R-3.13: instrumento Python fora da cópia");
        assert_eq!(v(&base(&ok.replace("\"x\"", "\"\""))).unwrap_err(), "R-2.1: condição de reprovação ausente");
        let dep = ok.replace("\"esperado\": 1", "\"esperado\": 1, \"depende_de\": [\"z\"]");
        assert_eq!(v(&base(&dep)).unwrap_err(), "R-2.12: dependências fora do candidato");
        for t in ["0", "true", "1.5", "null"] {
            let to = ok.replace("\"esperado\": 1", &format!("\"esperado\": 1, \"timeout_s\": {t}"));
            assert_eq!(v(&base(&to)).unwrap_err(), "R-5.3: timeout inválido");
        }
        assert_eq!(v(&base(&ok.replace("[\"execucao\"]", "[]"))).unwrap_err(), "R-2.8: instrumentos aceitos ausentes");
        // J-K4: regras de mutação fora da 0.1
        let mut_ = ok.replace("\"esperado\": 1", "\"regra\": \"sem_sobreviventes\"");
        assert_eq!(v(&base(&mut_)).unwrap_err(), "R-2.1: regra inválida");
        for ct in ["[]", "[{}]", "[{\"t.py\": \"x\"}]", "[{\"z\": \"x\"}]", "[{\"dados.txt\": 1}]", "{}"] {
            let cn = ok.replace("\"esperado\": 1", &format!("\"esperado\": 1, \"controles_negativos\": {ct}"));
            assert!(v(&base(&cn)).unwrap_err().starts_with("R-3.4: "), "{ct}");
        }
        let cn = ok.replace("\"esperado\": 1", "\"esperado\": 1, \"controles_negativos\": [{\"dados.txt\": null}]");
        assert!(v(&base(&cn)).is_ok());
        // lista de arquivos inválida
        assert!(validar_contrato(&j(&base(ok)), &["./t.py".to_string()]).unwrap_err().starts_with("J-C1: "));
    }

    #[test]
    fn contrato_gate_formato() {
        let c = contrato("");
        let g = contrato_gate(&c, "sha256:x").unwrap();
        assert_eq!(
            String::from_utf8(crate::json::json_canonico(&g)).unwrap(),
            r#"{"candidato":"sha256:x","evidencias_aceitas":{"t":["execucao"]},"integracao":{"evidencias_aceitas":["execucao"]},"obrigatorios":["t"],"severidade":{"t":"alta"},"versao":"v1"}"#
        );
        let mut o = c.como_objeto().unwrap().clone();
        o.inserir("premissas", Valor::Nulo);
        let g = contrato_gate(&Valor::Objeto(o), "sha256:x").unwrap();
        assert!(g.como_objeto().unwrap().contem("premissas"));
        assert!(contrato_gate(&j("{}"), "x").unwrap_err().starts_with("J-K2"));
    }
}

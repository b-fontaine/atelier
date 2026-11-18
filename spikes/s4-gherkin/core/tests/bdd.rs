use cucumber::{gherkin::Step, given, then, when, World};

#[derive(Debug, Default, World)]
struct NotesWorld {
    notes: Vec<(String, u32)>,
    coef: f64,
}

#[given("une liste de notes vide")]
fn vide(w: &mut NotesWorld) {
    w.notes.clear();
}

#[when(expr = "j'ajoute la note {string} avec la priorité {int}")]
fn ajoute(w: &mut NotesWorld, titre: String, prio: u32) {
    if !titre.is_empty() {
        w.notes.push((titre, prio));
    }
}

#[when("j'ajoute les notes suivantes:")]
fn ajoute_table(w: &mut NotesWorld, step: &Step) {
    let t = step.table.as_ref().expect("table");
    for row in t.rows.iter().skip(1) {
        w.notes.push((row[0].trim_matches('"').to_string(), row[1].parse().unwrap()));
    }
}

#[when(expr = "j'applique un coefficient de {float}")]
fn coef(w: &mut NotesWorld, c: f64) {
    w.coef = c;
}

#[then(expr = "la liste contient {int} note(s)")]
fn compte(w: &mut NotesWorld, n: usize) {
    assert_eq!(w.notes.len(), n);
}

#[then(expr = "la note {string} a la priorité {int}")]
fn prio(w: &mut NotesWorld, titre: String, p: u32) {
    assert!(w.notes.iter().any(|(t, q)| *t == titre && *q == p));
}

#[then(expr = "la priorité effective de {string} est {int}")]
fn effective(w: &mut NotesWorld, titre: String, p: u32) {
    let (_, q) = w.notes.iter().find(|(t, _)| *t == titre).unwrap();
    assert_eq!((*q as f64 * w.coef).round() as u32, p);
}

#[then(expr = "le statut est {word}")]
fn statut(_w: &mut NotesWorld, s: String) {
    assert_eq!(s, "ouvert");
}

#[tokio::main]
async fn main() {
    // Chemin des features : variable d'environnement, par défaut ../features
    let dir = std::env::var("FEATURES_DIR").unwrap_or_else(|_| "../features".into());
    NotesWorld::cucumber()
        .fail_on_skipped()
        .filter_run_and_exit(dir, |f, r, s| {
            let tag = std::env::var("CUCUMBER_TAG").unwrap_or_else(|_| "core".into());
            s.tags.iter().chain(r.iter().flat_map(|r| r.tags.iter())).chain(f.tags.iter()).any(|t| *t == tag)
        })
        .await;
}

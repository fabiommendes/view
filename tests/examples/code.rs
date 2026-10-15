// Contador de palavras simples, usado como exemplo de código Rust.
use std::collections::HashMap;

/// Conta a frequência de cada palavra em `text`.
pub fn word_freq(text: &str) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for word in text.split_whitespace() {
        // Remove pontuação nas bordas e normaliza para minúsculas.
        let key = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if key.is_empty() {
            continue;
        }
        *map.entry(key).or_insert(0) += 1;
    }
    map
}

fn describe(map: &HashMap<String, usize>) -> String {
    let total: usize = map.values().sum();
    format!("{} palavras distintas, {} no total", map.len(), total)
}

fn main() {
    let text = "Olá, mundo! O mundo é grande, o mundo é bom.";
    let freq = word_freq(text);
    println!("{}", describe(&freq));
    println!("tokens: \"mundo\" aparece {} vezes", freq["mundo"]);
}

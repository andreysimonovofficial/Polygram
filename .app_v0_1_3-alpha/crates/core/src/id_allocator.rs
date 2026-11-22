//! Генератор уникальных числовых ID объектов.
//!
//! ID объекта — случайное число в диапазоне от `0` до `999` (отображается как `#000`..`#999`).
//! Когда все ID текущего диапазона заняты, диапазон расширяется: `1000`..`9999`, затем `10000`..`99999` и так далее.
//! ID хранится как строка числа без приставки `#`; знак `#` добавляется только при отображении на холсте.
//! Первый созданный объект может получить любой ID из диапазона, например `#715`.

use std::collections::HashSet;
use rand::Rng;

/// Первая ширина диапазона ID: 3 цифры (0..=999).
const START_WIDTH: u32 = 3;

/// Выделить уникальный числовой ID, незанятый ни одним из `existing`.
/// Пытается подобрать случайный свободный ID в текущем диапазоне;
/// когда диапазон полностью исчерпан, переходит к следующему (большему).
pub fn allocate_id(existing: &[String]) -> String {
    let used: HashSet<String> = existing.iter().cloned().collect();
    let mut width: u32 = START_WIDTH;
    loop {
        let max: u32 = 10u32.pow(width).saturating_sub(1);
        let range_size = (max as usize) + 1;

        // Числовые ID из текущего диапазона, которые уже заняты.
        let used_in_range = used
            .iter()
            .filter_map(|id| id.parse::<u32>().ok())
            .filter(|&n| n <= max)
            .count();

        if used_in_range < range_size {
            // В диапазоне есть свободные ID — подбираем случайный свободный.
            let mut rng = rand::thread_rng();
            for _ in 0..10_000 {
                let n: u32 = rng.gen_range(0..=max);
                let id = n.to_string();
                if !used.contains(&id) {
                    return id;
                }
            }
            // Вероятностная защита: если не нашли за 10 000 попыток,
            // считаем диапазон исчерпанным и расширяем.
        }

        width += 1;
        if width > 8 {
            // Фолбэк на очень большое число: коллизия практически невозможна.
            return rand::thread_rng().gen::<u64>().to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_id_is_numeric_and_unique() {
        let mut existing: Vec<String> = Vec::new();
        for _ in 0..100 {
            let id = allocate_id(&existing);
            assert!(id.chars().all(char::is_numeric), "ID должен быть числом: {id}");
            assert!(!existing.contains(&id), "ID не должен дублироваться: {id}");
            existing.push(id);
        }
    }

    #[test]
    fn allocate_id_does_not_collide_with_non_numeric_existing() {
        let existing = vec!["dept_0".into(), "proc_1".into(), "d1".into(), "p".into()];
        let id = allocate_id(&existing);
        assert!(id.chars().all(char::is_numeric));
        assert!(!existing.contains(&id));
    }

    #[test]
    fn allocate_id_expands_when_range_full() {
        // Занимаем весь диапазон 0..=999.
        let existing: Vec<String> = (0..=999u32).map(|n| n.to_string()).collect();
        let id = allocate_id(&existing);
        let n: u32 = id.parse().expect("ID должен быть числом");
        assert!(n >= 1000, "при полном диапазоне 0..=999 должен быть выдан ID >= 1000, получен {id}");
    }
}

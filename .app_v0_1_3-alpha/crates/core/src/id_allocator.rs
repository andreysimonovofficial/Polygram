//! Генератор уникальных числовых ID объектов с ведущими нулями.
//!
//! ID объекта — случайное число, отображаемое как `#000`..`#999` (3 цифры с ведущими нулями).
//! Когда все 1000 ID диапазона `#000`..`#999` заняты, диапазон расширяется: `#1000`..`#9999`,
//! затем `#10000`..`#99999` и так далее.
//! ID хранится как строка числа с ведущими нулями без приставки `#`; знак `#` добавляется
//! только при отображении на холсте (`format!("#{}", object.id)`).
//! Первый созданный объект может получить любой ID из диапазона, например `#715` (хранится как `"715"`),
//! а `#7` хранится как `"007"`.

use std::collections::HashSet;
use rand::Rng;

/// Первая ширина диапазона ID: 3 цифры (0..=999).
const START_WIDTH: u32 = 3;

/// Минимальное значение для заданной ширины диапазона.
/// Ширина 3 — от 0, ширина 4 и больше — от 10^(width-1) (чтобы не заезжать на уже занятые
/// короткие ID после расширения диапазона).
fn range_min(width: u32) -> u32 {
    if width == START_WIDTH {
        0
    } else {
        10u32.pow(width - 1)
    }
}

/// Выделить уникальный числовой ID, незанятый ни одним из `existing`.
/// ID возвращается в виде строки с ведущими нулями длиной `width` (например `"007"`, `"1000"`).
pub fn allocate_id(existing: &[String]) -> String {
    let used: HashSet<String> = existing.iter().cloned().collect();
    let mut width: u32 = START_WIDTH;
    loop {
        let min = range_min(width);
        let max: u32 = 10u32.pow(width).saturating_sub(1);
        let range_size = (max as usize) - (min as usize) + 1;

        // Числовые ID из текущего диапазона, которые уже заняты.
        let used_in_range = used
            .iter()
            .filter_map(|id| id.parse::<u32>().ok())
            .filter(|&n| n >= min && n <= max)
            .count();

        if used_in_range < range_size {
            // В диапазоне есть свободные ID — подбираем случайный свободный.
            let mut rng = rand::thread_rng();
            for _ in 0..10_000 {
                let n: u32 = rng.gen_range(min..=max);
                let id = format!("{:0width$}", n, width = width as usize);
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
    fn allocate_id_is_zero_padded_and_unique() {
        let mut existing: Vec<String> = Vec::new();
        for _ in 0..100 {
            let id = allocate_id(&existing);
            assert!(
                id.chars().all(char::is_numeric),
                "ID должен быть числом: {id}"
            );
            assert!(
                id.len() >= START_WIDTH as usize,
                "ID должен иметь как минимум {START_WIDTH} цифр: {id}"
            );
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
        // Занимаем весь диапазон 0..=999 (хранится как "000".."999").
        let existing: Vec<String> = (0..=999u32).map(|n| format!("{:03}", n)).collect();
        let id = allocate_id(&existing);
        let n: u32 = id.parse().expect("ID должен быть числом");
        assert!(
            n >= 1000,
            "при полном диапазоне 0..=999 должен быть выдан ID >= 1000, получен {id}"
        );
        assert_eq!(id.len(), 4, "ID после расширения должен иметь 4 цифры, получен {id}");
    }

    #[test]
    fn allocate_id_formats_with_leading_zeros_when_possible() {
        // Форматирование с ведущими нулями проверяется напрямую на границе диапазонов.
        // Когда диапазон 0..=999 полностью заполнен, выдаётся 4-значный ID.
        let existing: Vec<String> = (0..=999u32).map(|n| format!("{:03}", n)).collect();
        let id = allocate_id(&existing);
        assert_eq!(id.len(), 4, "ожидается 4-значный ID, получен {id}");
    }
}

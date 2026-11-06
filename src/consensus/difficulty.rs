//! Логика сложности и накопленной работы цепочки перенесена из `blockchain::difficulty`
//! без изменения вычислений, констант или поведения консенсуса.
//!
//! Сложность блока задаёт количество ведущих нулевых hex-символов, которое должен
//! иметь хеш его заголовка. Каждый дополнительный нулевой hex-символ уменьшает
//! вероятность подходящего хеша в 16 раз, поэтому ожидаемая работа блока равна
//! `16 ^ difficulty`.
//!
//! Сложность пересчитывается не после каждого блока, а только на границе заданного
//! интервала. Алгоритм сравнивает фактическое время формирования окна блоков с
//! целевым временем и изменяет сложность максимум на один шаг.

use num_bigint::BigUint;

use crate::{block::Block, blockchain::error::BlockValidationError};

use super::{
    ConsensusParams, DEFAULT_DIFFICULTY_ADJUSTMENT_INTERVAL, DEFAULT_TARGET_BLOCK_TIME, Difficulty,
};

/// Целевое время между соседними блоками в секундах.
///
/// Значение используется как настройка по умолчанию в `ConsensusParams`.
pub const TARGET_BLOCK_TIME: u64 = DEFAULT_TARGET_BLOCK_TIME;

/// Количество блоков между точками перерасчёта сложности.
///
/// При значении `10` новая сложность рассчитывается перед блоками с индексами
/// `10`, `20`, `30` и так далее. Между этими точками сложность предыдущего блока
/// переносится без изменений.
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = DEFAULT_DIFFICULTY_ADJUSTMENT_INTERVAL;

/// Рассчитывает сложность следующего блока логикой, перенесённой из
/// `blockchain::difficulty::expected_difficulty_for_next_block`.
///
/// Алгоритм работает следующим образом:
///
/// 1. Определяет индекс следующего блока по вершине цепочки.
/// 2. Если индекс не находится на границе интервала перерасчёта, возвращает
///    сложность последнего блока без изменений.
/// 3. На границе интервала берёт окно из `difficulty_adjustment_interval` блоков
///    и измеряет время между первым и последним блоком окна.
/// 4. Сравнивает фактическое время с целевым:
///    - быстрее половины целевого времени — увеличивает сложность на один;
///    - медленнее удвоенного целевого времени — уменьшает сложность на один;
///    - в остальных случаях сохраняет текущую сложность.
///
/// Изменение ограничено одним шагом, нулём снизу и `max_difficulty` сверху, чтобы
/// одно аномальное окно не вызвало резкого скачка параметров Proof-of-Work.
pub fn expected_difficulty_for_next_block(
    chain: &[Block],
    params: &ConsensusParams,
) -> Result<Difficulty, BlockValidationError> {
    // Для пустой цепочки невозможно определить ни предыдущую сложность, ни индекс.
    let last_block = chain.last().ok_or(BlockValidationError::MissingChainTip)?;

    // Индекс нужен для определения границы интервала; checked_add не допускает
    // перехода индекса через `u64::MAX`.
    let next_block_index = last_block
        .header
        .index
        .checked_add(1)
        .ok_or(BlockValidationError::IndexOverflow)?;
    let adjustment_interval = params.difficulty_adjustment_interval();

    // До следующей точки перерасчёта все блоки наследуют текущую сложность.
    if next_block_index % adjustment_interval != 0 {
        return Ok(last_block.header.difficulty);
    };

    // Для перерасчёта требуется полное окно истории. Например, перед блоком 10
    // используются блоки 0..=9, то есть десять блоков и девять временных промежутков.
    let required_history = adjustment_interval as usize;
    let start_position = chain.len().checked_sub(required_history).ok_or(
        BlockValidationError::InsufficientDifficultyHistory {
            required: required_history,
            actual: chain.len(),
        },
    )?;

    // `start_position` указывает на первый блок текущего окна перерасчёта.
    let start =
        chain
            .get(start_position)
            .ok_or(BlockValidationError::InsufficientDifficultyHistory {
                required: required_history,
                actual: chain.len(),
            })?;

    // Фактическая длительность окна измеряется по timestamp первого и последнего
    // блоков. checked_sub также защищает от обратного хода времени.
    let actual_time = last_block
        .header
        .timestamp
        .checked_sub(start.header.timestamp)
        .ok_or(BlockValidationError::TimestampBeforePrevious {
            previous: start.header.timestamp,
            actual: last_block.header.timestamp,
        })?;

    // В окне из N блоков находится N - 1 промежутков между соседними блоками,
    // поэтому целевое время умножается именно на `adjustment_interval - 1`.
    let expected_time = params.target_block_time() * (adjustment_interval - 1);

    // Блоки создавались более чем в два раза быстрее цели: увеличиваем сложность.
    // saturating_add защищает от переполнения, а min — от превышения consensus-лимита.
    if actual_time < expected_time / 2 {
        return Ok(last_block
            .header
            .difficulty
            .saturating_add(1)
            .min(params.max_difficulty()));
    }

    // Блоки создавались более чем в два раза медленнее цели: уменьшаем сложность.
    // saturating_sub не позволяет значению стать меньше нуля.
    if actual_time > expected_time * 2 {
        return Ok(last_block.header.difficulty.saturating_sub(1));
    }

    // В допустимом диапазоне от половины до удвоенного целевого времени
    // корректировка не требуется.
    Ok(last_block.header.difficulty)
}

/// Возвращает ожидаемый объём работы для заданной сложности.
///
/// Функция перенесена из `blockchain::difficulty::block_work`. Один ведущий
/// нулевой hex-символ имеет вероятность `1 / 16`, поэтому каждый следующий уровень
/// сложности умножает ожидаемое число попыток на `16`. `BigUint` используется,
/// поскольку `16 ^ difficulty` может не помещаться в примитивный целочисленный тип.
pub fn block_work(difficulty: Difficulty) -> BigUint {
    BigUint::from(16u32).pow(difficulty as u32)
}

/// Суммирует ожидаемую работу всех блоков цепочки.
///
/// Функция перенесена из `blockchain::difficulty::chain_work`. Суммарная работа,
/// в отличие от простого количества блоков, позволяет сравнивать цепочки, блоки
/// которых были добыты с разной сложностью.
pub fn chain_work(chain: &[Block]) -> BigUint {
    chain.iter().fold(BigUint::from(0u32), |acc, block| {
        acc + block_work(block.header.difficulty)
    })
}

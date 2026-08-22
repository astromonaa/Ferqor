use crate::blockchain::utils::Difficulty;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    /* неправильная подпись или публичный ключ */
    InvalidTransaction,

    /* неправильная последовательность */
    InvalidNonce { expected: u64, actual: u64 },

    /* переполнение amount + fee */
    CostOverflow,

    /* переполнение баланса получателя или майнера */
    BalanceOverflow,

    /* невозможно увеличить nonce */
    NonceOverflow,

    /* недостаточно средств */
    InsufficientBalance { available: u64, required: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockValidationError {
    /// В цепочке отсутствует последний блок.
    ///
    /// Без текущей вершины невозможно определить ожидаемый индекс
    /// и `previous_hash` принимаемого блока.
    MissingChainTip,

    /// Индекс последнего блока равен `u64::MAX`,
    /// поэтому вычислить следующий индекс через `checked_add(1)` невозможно.
    IndexOverflow,

    /// Индекс принимаемого блока не совпадает со следующим ожидаемым индексом.
    UnexpectedIndex {
        /// Индекс, который должен иметь следующий блок.
        expected: u64,
        /// Индекс, указанный в заголовке принимаемого блока.
        actual: u64,
    },

    /// `previous_hash` принимаемого блока не совпадает
    /// с хешем текущей вершины цепочки.
    PreviousHashMismatch,

    /// Хеш, сохранённый в `block.hash`, не совпадает
    /// с повторно вычисленным хешем заголовка.
    HashMismatch,

    /// Хеш блока не удовлетворяет текущей сложности Proof of Work.
    InvalidProofOfWork,

    /// `merkle_root` из заголовка не совпадает с корнем,
    /// повторно вычисленным по транзакциям блока.
    MerkleRootMismatch,

    /// При суммировании веса транзакций блока
    /// произошло переполнение `usize`.
    WeightOverflow,

    /// Вес блока превышает установленный сетевой лимит.
    BlockTooHeavy {
        /// Фактически вычисленный вес блока.
        weight: usize,
        /// Максимально допустимый вес блока.
        max_weight: usize,
    },

    /// Coinbase-транзакция нарушает одно из правил:
    ///
    /// - отсутствует;
    /// - находится не на первой позиции;
    /// - в блоке находится больше одной coinbase;
    /// - награда не равна `mining_reward + total_fees`;
    /// - при вычислении награды или комиссий произошло переполнение.
    InvalidCoinbaseTransaction,

    UnexpectedDifficulty {
        expected: Difficulty,
        actual: Difficulty,
    },
    
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockAcceptanceError {
    /// Блок не прошёл структурную проверку.
    ///
    /// Состояние блокчейна при этом не должно изменяться.
    Validation(BlockValidationError),

    /// Одна из транзакций блока не смогла примениться
    /// к временному состоянию аккаунтов.
    StateTransition {
        /// Позиция некорректной транзакции в `block.transactions`.
        ///
        /// Coinbase находится на позиции `0`.
        transaction_index: usize,
        /// Конкретная причина ошибки применения транзакции.
        source: TransactionError,
    },
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiningError {
    BlockCapacityTooSmall {
        required: usize,
        max_weight: usize,
    },

    FeeOverflow,

    RewardOverflow,

    BlockAcceptance(BlockAcceptanceError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockchainError {
    InvalidDifficulty,
    Mining(MiningError),
}
//! Параметры консенсуса, объединённые из полей `Blockchain`,
//! `config::GenesisConfig`, `blockchain::difficulty` и `blockchain::utils`.
//!
//! Эти значения влияют на допустимость блоков, размер награды и перерасчёт
//! сложности. Создание блока и его последующая валидация обязаны использовать один
//! и тот же набор параметров, иначе узлы могут по-разному оценить одну цепочку.
//!
//! `ConsensusParams` хранится внутри `Chain` и после создания цепочки доступен только
//! для чтения. Это не позволяет случайно изменить правила для уже существующей цепочки.

/// Количество ведущих нулевых hex-символов, требуемых от хеша блока.
///
/// Тип перенесён из `blockchain::utils::Difficulty`. Значение `0` разрешает любой
/// хеш, а увеличение значения на единицу повышает ожидаемую работу в 16 раз.
pub type Difficulty = u8;

/// Максимально допустимая сложность.
///
/// Хеш блока состоит из 32 байт, то есть из 64 hex-символов. Поэтому требовать
/// больше 64 ведущих нулей невозможно.
pub const MAX_DIFFICULTY: Difficulty = 64;

/// Базовая награда майнеру за один блок без учёта комиссий транзакций.
///
/// При создании coinbase-транзакции к этому значению добавляется сумма комиссий
/// остальных транзакций блока. Валидатор проверяет точное равенство этой сумме.
pub const DEFAULT_MINING_REWARD: u64 = 50;

/// Максимальный суммарный вес блока по умолчанию.
///
/// Вес блока складывается из веса всех его транзакций. В текущей модели каждая
/// транзакция имеет `Transaction::FIXED_WEIGHT = 1`, включая coinbase-транзакцию.
pub const DEFAULT_MAX_BLOCK_WEIGHT: usize = 1000;

/// Целевое время между соседними блоками в секундах.
///
/// Значение не ограничивает время создания отдельного блока напрямую, а участвует
/// в сравнении ожидаемой и фактической длительности окна перерасчёта сложности.
pub const DEFAULT_TARGET_BLOCK_TIME: u64 = 10;

/// Количество блоков между точками перерасчёта сложности.
///
/// При значении `10` перерасчёт выполняется перед блоками с индексами `10`, `20`,
/// `30` и так далее. Между этими точками сложность предыдущего блока сохраняется.
pub const DEFAULT_DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 10;

/// Неизменяемый набор правил, влияющих на создание и валидацию блоков.
///
/// Значения ранее были распределены между `Blockchain`, `GenesisConfig`,
/// `blockchain::difficulty` и `blockchain::utils`. Их объединение гарантирует, что
/// майнинг и проверка цепочки получают согласованные параметры из одного источника.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsensusParams {
    /// Базовая часть coinbase-награды, к которой добавляются комиссии блока.
    mining_reward: u64,
    /// Верхняя граница суммы весов всех транзакций блока, включая coinbase.
    max_block_weight: usize,
    /// Желаемое среднее время между соседними блоками в секундах.
    target_block_time: u64,
    /// Число блоков между последовательными перерасчётами сложности.
    difficulty_adjustment_interval: u64,
    /// Верхняя граница сложности, защищающая от недостижимого требования к хешу.
    max_difficulty: Difficulty,
}

impl ConsensusParams {
    /// Возвращает базовую награду из прежнего поля `Blockchain::mining_reward`.
    ///
    /// Итоговая coinbase-награда формируется отдельно как базовая награда плюс
    /// сумма комиссий всех обычных транзакций блока.
    pub const fn mining_reward(&self) -> u64 {
        self.mining_reward
    }

    /// Возвращает лимит веса из прежнего поля `Blockchain::max_block_weight`.
    ///
    /// Этот лимит используется и при выборе транзакций из mempool, и при полной
    /// валидации блока, поэтому майнер не должен создавать заведомо тяжёлый блок.
    pub const fn max_block_weight(&self) -> usize {
        self.max_block_weight
    }

    /// Возвращает прежнее значение `blockchain::difficulty::TARGET_BLOCK_TIME`.
    ///
    /// Алгоритм сложности умножает его на количество временных промежутков в окне,
    /// чтобы получить ожидаемую длительность этого окна.
    pub const fn target_block_time(&self) -> u64 {
        self.target_block_time
    }

    /// Возвращает прежнее значение
    /// `blockchain::difficulty::DIFFICULTY_ADJUSTMENT_INTERVAL`.
    ///
    /// Значение определяет одновременно частоту перерасчёта и размер необходимого
    /// окна истории блоков.
    pub const fn difficulty_adjustment_interval(&self) -> u64 {
        self.difficulty_adjustment_interval
    }

    /// Возвращает прежнее значение `blockchain::utils::MAX_DIFFICULTY`.
    ///
    /// При повышении сложности результат ограничивается этим значением, чтобы число
    /// требуемых нулевых hex-символов не превышало длину хеша.
    pub const fn max_difficulty(&self) -> Difficulty {
        self.max_difficulty
    }

    /// Заменяет только лимит веса значением из `GenesisConfig`.
    ///
    /// Метод принимает `self` по значению и возвращает новый набор параметров,
    /// сохраняя награду, параметры сложности и остальные значения по умолчанию.
    pub(crate) const fn with_max_block_weight(mut self, max_block_weight: usize) -> Self {
        self.max_block_weight = max_block_weight;
        self
    }
}

impl Default for ConsensusParams {
    /// Собирает исходные consensus-значения, которые использовались до их
    /// объединения в `ConsensusParams`.
    fn default() -> Self {
        Self {
            mining_reward: DEFAULT_MINING_REWARD,
            max_block_weight: DEFAULT_MAX_BLOCK_WEIGHT,
            target_block_time: DEFAULT_TARGET_BLOCK_TIME,
            difficulty_adjustment_interval: DEFAULT_DIFFICULTY_ADJUSTMENT_INTERVAL,
            max_difficulty: MAX_DIFFICULTY,
        }
    }
}

use crate::address::Address;
use crate::transaction::{Transaction, TransactionId};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::sync::Arc;

/// Основное хранилище транзакций, позволяющее получить транзакцию по её идентификатору.
///
/// `Arc` позволяет возвращать выбранные для блока транзакции без копирования самих
/// `Transaction` и при этом сохранять их в mempool до успешного добавления блока.
pub type TransactionMap = HashMap<TransactionId, Arc<Transaction>>;

/// Вторичный индекс транзакций по отправителю и nonce.
///
/// Вложенный `BTreeMap` хранит nonce в упорядоченном виде и обеспечивает для одного
/// отправителя не более одной транзакции с конкретным nonce. Значение содержит только
/// `TransactionId`, а полная транзакция хранится в `TransactionMap`.
pub type SenderMap = HashMap<Address, BTreeMap<u64, TransactionId>>;

/// Кандидат на включение в блок: `(комиссия, nonce, идентификатор транзакции)`.
///
/// `BinaryHeap` в Rust является max-heap и сравнивает элементы кортежа слева направо.
/// Поэтому наибольшая комиссия имеет первый приоритет. `Reverse<u64>` инвертирует
/// сравнение nonce: при одинаковой комиссии меньший nonce извлекается раньше, хотя
/// обычный `u64` дал бы приоритет большему значению. `TransactionId` служит последним
/// детерминированным критерием, если комиссия и nonce совпали.
type MineCandidate = (u64, Reverse<u64>, TransactionId);

/// Пул ожидающих включения в блок транзакций.
///
/// Mempool поддерживает два синхронных представления данных:
/// - `transactions` хранит полные транзакции по `txid`;
/// - `by_sender` связывает пару `(отправитель, nonce)` с тем же `txid`.
///
/// Методы изменения должны обновлять оба индекса вместе. Нарушение этого инварианта
/// рассматривается как неконсистентное внутреннее состояние.
pub struct Mempool {
    /// Полные транзакции, индексированные по их идентификаторам.
    pub transactions: TransactionMap,
    /// Вторичный индекс для проверки nonce и последовательного выбора транзакций отправителя.
    pub by_sender: SenderMap,
}

impl Mempool {
    /// Создаёт пустой mempool с согласованными пустыми индексами.
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            by_sender: HashMap::new(),
        }
    }

    /// Проверяет наличие транзакции в основном хранилище по её идентификатору.
    pub fn contains(&self, txid: &TransactionId) -> bool {
        self.transactions.contains_key(txid)
    }

    /// Возвращает количество ожидающих транзакций конкретного отправителя.
    ///
    /// Если отправитель ещё не представлен во вторичном индексе, его очередь считается
    /// пустой. Преобразование в `u64` соответствует типу nonce и прежнему публичному API.
    pub fn pending_count(&self, sender: &Address) -> u64 {
        self.by_sender
            .get(sender)
            .map(|txs| txs.len() as u64)
            .unwrap_or(0)
    }

    /// Рассчитывает общую сумму `amount + fee` всех ожидающих транзакций отправителя.
    ///
    /// Возвращает `None`, если арифметика `u64` переполняется либо два внутренних индекса
    /// расходятся: транзакция отсутствует в `transactions`, принадлежит другому отправителю
    /// или содержит nonce, отличный от ключа во вторичном индексе. Для отправителя без
    /// ожидающих транзакций стоимость равна нулю.
    pub fn pending_cost(&self, sender: &Address) -> Option<u64> {
        let Some(sender_map) = self.by_sender.get(sender) else {
            return Some(0);
        };

        // `try_fold` немедленно возвращает `None` при нарушении инварианта или переполнении.
        sender_map.iter().try_fold(0u64, |acc, (nonce, txid)| {
            let tx = self.transactions.get(txid)?;

            // Пара `(sender, nonce)` во вторичном индексе должна описывать ту же транзакцию.
            if tx.from != *sender || tx.nonce != *nonce {
                return None;
            }

            // Обе операции проверяются отдельно, чтобы переполнение никогда не оборачивалось.
            let amount = tx.amount.checked_add(tx.fee)?;
            acc.checked_add(amount)
        })
    }

    /// Добавляет транзакцию одновременно в основное хранилище и индекс отправителя.
    ///
    /// Транзакция отклоняется, если уже существует тот же `txid` либо отправитель уже
    /// имеет другую ожидающую транзакцию с таким nonce. Сначала выполняются все проверки,
    /// и только затем обновляются оба индекса, поэтому обычный отказ не оставляет частичных
    /// изменений.
    pub fn insert(&mut self, transaction: Transaction) -> bool {
        let txid = transaction.txid();

        if self.transactions.contains_key(&txid) {
            return false;
        }

        let sender_map = self.by_sender.entry(transaction.from).or_default();

        if sender_map.contains_key(&transaction.nonce) {
            return false;
        }

        // Вторичный индекс хранит только идентификатор, исключая дублирование Transaction.
        sender_map.insert(transaction.nonce, txid);

        self.transactions.insert(txid, Arc::new(transaction));

        true
    }

    /// Удаляет транзакцию из обоих индексов.
    ///
    /// Перед изменением проверяется, что вторичный индекс действительно указывает на тот
    /// же `txid`. Если один из индексов уже неконсистентен, метод возвращает `false` и не
    /// удаляет данные только из одного представления.
    pub fn remove(&mut self, txid: &TransactionId) -> bool {
        // Копируем ключи до получения изменяемой ссылки на `by_sender`, чтобы не держать
        // заимствование `transactions` во время обновления обоих индексов.
        let (sender, nonce) = {
            let Some(tx) = self.transactions.get(txid) else {
                return false;
            };
            (tx.from, tx.nonce)
        };

        let Some(sender_map) = self.by_sender.get_mut(&sender) else {
            return false;
        };

        let Some(found_txid) = sender_map.get(&nonce) else {
            return false;
        };

        if found_txid != txid {
            return false;
        }

        self.transactions.remove(txid);
        sender_map.remove(&nonce);

        // Пустые карты отправителей не хранятся, чтобы `by_sender` отражал только тех,
        // у кого действительно есть ожидающие транзакции.
        if sender_map.is_empty() {
            self.by_sender.remove(&sender);
        }

        true
    }

    /// Добавляет в очередь приоритетов транзакцию отправителя с ожидаемым nonce.
    ///
    /// Отсутствие nonce означает разрыв последовательности: более поздние транзакции этого
    /// отправителя пока нельзя включать в блок. Отсутствие `txid` в основном хранилище
    /// означает нарушение инварианта индексов; такой кандидат также не добавляется.
    fn push_candidate(
        &self,
        heap: &mut BinaryHeap<MineCandidate>,
        sender_map: &BTreeMap<u64, TransactionId>,
        nonce: u64,
    ) {
        let Some(transaction_id) = sender_map.get(&nonce) else {
            return;
        };

        let Some(transaction) = self.transactions.get(transaction_id) else {
            return;
        };

        // Приоритет задаётся самим кортежем: сначала максимальная fee, затем минимальный
        // nonce благодаря `Reverse`, затем `TransactionId` как последний критерий.
        heap.push((transaction.fee, Reverse(nonce), *transaction_id))
    }

    /// Выбирает транзакции для нового блока в пределах его доступного веса.
    ///
    /// `account_nonces` содержит следующий допустимый nonce каждого аккаунта из текущего
    /// состояния реестра. Для каждого отправителя сначала рассматривается только транзакция
    /// с этим nonce. Следующая транзакция того же отправителя становится кандидатом лишь
    /// после выбора предыдущей, поэтому в блок не попадает последовательность с пропуском.
    ///
    /// Среди доступных в конкретный момент кандидатов преимущество получает наибольшая
    /// комиссия. Метод не удаляет выбранные транзакции из mempool: это делается отдельно
    /// после успешного принятия блока.
    pub fn select_transactions(
        &self,
        max_block_weight: usize,
        account_nonces: &HashMap<Address, u64>,
    ) -> Vec<Arc<Transaction>> {
        let mut selected = Vec::new();
        let mut binary_heap = BinaryHeap::new();

        // Формируем начальный фронт: не более одной исполнимой транзакции от отправителя.
        for (sender, sender_map) in &self.by_sender {
            let expected_nonce = account_nonces.get(sender).copied().unwrap_or(0);

            self.push_candidate(&mut binary_heap, sender_map, expected_nonce);
        }

        let mut current_weight: usize = 0;

        // `Reverse(nonce)` в шаблоне извлекает исходный `u64` из обёртки после pop.
        while let Some((_, Reverse(nonce), txid)) = binary_heap.pop() {
            let Some(best_transaction) = self.transactions.get(&txid) else {
                continue;
            };

            let tx_weight = best_transaction.weight();

            // Переполнение веса трактуется так же, как невозможность вместить кандидата.
            let Some(next_weight) = current_weight.checked_add(tx_weight) else {
                continue;
            };

            if next_weight > max_block_weight {
                continue;
            };

            current_weight = next_weight;

            // Клонируется только `Arc`, а сама транзакция остаётся общей с mempool.
            selected.push(Arc::clone(best_transaction));

            // Только после включения текущей транзакции можно открыть следующую транзакцию
            // этого же отправителя. При `u64::MAX` следующего корректного nonce не существует.
            let Some(next_nonce) = nonce.checked_add(1) else {
                continue;
            };

            let Some(sender_map) = self.by_sender.get(&best_transaction.from) else {
                continue;
            };

            self.push_candidate(&mut binary_heap, sender_map, next_nonce);
        }

        selected
    }

    /// Удаляет набор транзакций, уже принятых в блок, используя единичную операцию
    /// `remove`, чтобы для каждой транзакции согласованно обновить оба индекса.
    pub fn remove_selected_transactions(&mut self, transactions: &[Arc<Transaction>]) {
        for transaction in transactions {
            self.remove(&transaction.txid());
        }
    }
}

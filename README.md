# Ferqor

Ferqor — блокчейн на Rust, находящийся на этапе активной разработки. Проект включает Proof-of-Work, подписанные транзакции, управление состоянием аккаунтов, mempool, каноническое бинарное кодирование и развивающийся асинхронный P2P-слой.

> Проект ещё не готов к production-использованию: сетевой протокол и синхронизация цепочки продолжают развиваться, а данные пока хранятся только в памяти.

## Возможности

- **Proof-of-Work** — майнинг блоков и проверка количества ведущих нулей в SHA-256 хеше.
- **Динамическая сложность** — перерасчёт сложности на основе времени создания предыдущих блоков.
- **Транзакции Ed25519** — подпись, проверка публичного ключа и защита от изменения полей транзакции.
- **Состояние аккаунтов** — балансы и последовательные nonce с проверкой переполнений.
- **Mempool** — индексация по отправителю и nonce, учёт pending-баланса и выбор транзакций по комиссии в пределах веса блока.
- **Coinbase** — вознаграждение майнера с включением комиссий выбранных транзакций.
- **Merkle tree** — детерминированный корень транзакций с domain-separated кодированием узлов.
- **Канонические кодеки** — стабильное big-endian кодирование транзакций, блоков и сетевых сообщений.
- **Безопасное декодирование** — контроль границ буфера, trailing bytes и лимитов размера блока/транзакции.
- **TCP P2P-слой** — фрейминг сообщений, handshake, отдельные reader/writer-задачи и маршрутизация через Tokio channels.
- **Валидация блоков** — проверка индекса, previous hash, Merkle root, веса, timestamp, сложности, PoW, coinbase и переходов состояния.

## Требования

- Rust с поддержкой edition 2024
- Cargo

## Установка и запуск

```bash
git clone https://github.com/astromonaa/Ferqor.git
cd Ferqor
cp .env.example .env.local
cargo build --release
cargo run --release -- --env-file=.env.local
```

`main` загружает runtime-конфигурацию из указанного env-файла и совместно запускает TCP Node, Application и BlockchainService. Локальные `.env`-файлы игнорируются Git; для новой конфигурации используйте `.env.example` как шаблон.

### Runtime-конфигурация

- `LISTEN_ADDR` — локальный адрес TCP listener, по умолчанию `0.0.0.0:9000`;
- `ADVERTIZE_ADDR` — публичный адрес узла, отправляемый в handshake;
- `NODE_ID` — обязательный уникальный идентификатор из 32 байт в hex-представлении;
- `CONNECT` — необязательный список адресов peer через запятую;
- `INITIAL_SENDER` — включает локальный demo-сценарий: после запуска отправляется транзакция и команда майнинга.

Для двух локальных узлов создайте два env-файла с разными `LISTEN_ADDR`, `ADVERTIZE_ADDR` и `NODE_ID`. У второго узла укажите адрес первого в `CONNECT`. Не используйте демонстрационные ключи и идентификаторы в публичной сети.

## Пример использования блокчейна

```rust
use std::collections::HashMap;

use crate::blockchain::Blockchain;
use crate::config::GenesisConfig;
use crate::wallet::Wallet;

fn main() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let genesis = GenesisConfig::get_config(
        Some(HashMap::from([(alice.address, 100)])),
        None, // лимит веса блока по умолчанию
        4,    // начальная сложность
    )
    .expect("valid genesis configuration");

    let mut blockchain =
        Blockchain::new(genesis).expect("valid genesis block");

    let transaction =
        alice.create_transaction(bob.address, 50, 5, 0);

    assert!(blockchain.add_transaction(transaction));

    let Some((mut candidate, next_ledger)) = blockchain
        .mine_pending_transactions(alice.address)
        .expect("candidate creation should succeed")
    else {
        return;
    };

    candidate.mine_block();
    blockchain
        .add_block(candidate, next_ledger)
        .expect("mined block should be accepted");

    assert!(blockchain.is_valid());
}
```

## Архитектура

### Blockchain, Chain и Ledger

`Blockchain` в `src/blockchain/` — публичный фасад, координирующий каноническую цепочку, mempool и создание блоков-кандидатов.

`src/chain/` владеет блоками, текущим и начальным состоянием, параметрами консенсуса и валидацией цепочки. Основа реестра индексированных блоков уже выделена, но полноценный fork choice и reorg ещё не завершены.

`src/ledger/` отвечает только за balances, nonce и атомарные переходы состояния. Транзакции блока применяются к копии `LedgerState`; текущее состояние заменяется лишь при успешном commit блока.

### Consensus и Mining

`src/consensus/` содержит неизменяемые параметры консенсуса, перерасчёт сложности и оценку накопленной работы. Создание и валидация блоков используют один набор `ConsensusParams`.

`src/mining/` создаёт блоки-кандидаты и coinbase-транзакции. Proof-of-Work выполняется исходным `Block::mine_block` вне владельца состояния, после чего добытый кандидат возвращается в `BlockchainService` для валидации и commit.

### Block и Transaction

`src/block/` и `src/transaction/` определяют структуры данных и их канонические кодеки. Consensus-critical commitment вынесены в отдельные внутренние модули, чтобы txid, подписи, Merkle root и block hash использовали единое domain-separated представление байтов.

### Mempool

`src/mempool/` хранит транзакции по ID и индексирует их по отправителю и nonce. При формировании блока выбираются только последовательные транзакции, а суммарный вес не превышает лимит после резервирования места под coinbase.

### Network

`src/network/` реализует асинхронный TCP-транспорт на Tokio:

- заголовок frame `P2P_MSG_V1`;
- сообщения `Transaction`, `Block` и `Handshake`;
- лимиты декодирования для защиты от чрезмерных payload;
- handshake с проверкой версии протокола, network ID и self-connection;
- отдельные reader/writer loops для каждого peer;
- реестр peer-соединений, connection ID, лимиты подключений и повторные попытки соединения;
- команды `SendTo`, `Broadcast`, `BroadcastExcept`, `Disconnect` и `Shutdown`.

### App, BlockchainService и Runtime

`App` маршрутизирует сообщения между Node, локальными командами и `BlockchainService`. Принятые транзакции и блоки передаются сервису, а успешно обработанные данные отправляются обратно в Node для broadcast.

`BlockchainService` является единственным владельцем изменяемого `Blockchain`. Подготовленный блок-кандидат добывается в отдельном потоке, затем возвращается сервису и фиксируется после валидации.

`src/runtime/` создаёт Tokio channels и конкурентно запускает `Node`, `App` и `BlockchainService`.

## Сетевой формат

Сетевой frame состоит из:

1. тега `P2P_MSG_V1`;
2. одного байта типа сообщения;
3. четырёх байтов длины payload в big-endian;
4. payload сообщения.

До выделения буфера транспорт проверяет тип сообщения и соответствующий лимит размера. Handshake передаёт версию протокола, network ID, node ID и публичный адрес узла.

## Proof-of-Work и сложность

Хеш заголовка блока должен иметь заданное число ведущих нулевых hex-символов. Сложность хранится непосредственно в заголовке и входит в его хеш.

Через заданный интервал блоков сложность корректируется по фактическому времени их создания относительно целевого времени блока. Для сравнения цепочек предусмотрен расчёт накопленной работы через `num-bigint`.

## Структура проекта

```text
src/
├── address/       # Адреса и балансы
├── app/           # App, команды и владелец BlockchainService
├── block/         # Блоки, Merkle tree, commitment и кодек
├── blockchain/    # Публичный фасад Blockchain и ошибки
├── chain/         # Каноническая цепочка, индекс и валидация
├── codec/         # Безопасный ByteReader и ошибки декодирования
├── config/        # Genesis, runtime-конфигурация и сетевые лимиты
├── consensus/     # Параметры, сложность и накопленная работа
├── ledger/        # Балансы, nonce и переходы состояния
├── mempool/       # Очередь неподтверждённых транзакций
├── mining/        # Создание кандидатов и coinbase
├── network/       # TCP transport, wire protocol, Node и Peer
├── runtime/       # Совместный запуск Node, App и BlockchainService
├── transaction/   # Транзакции, commitment и канонический кодек
├── utils/         # Локальные команды demo-сценария
├── wallet/        # Ed25519-кошелёк
└── main.rs        # Загрузка конфигурации и запуск runtime
```

## Тесты

```bash
cargo test
```

Текущий набор покрывает подписи и канонические commitment транзакций, блоковые кодеки и Merkle root, nonce, pending cost, переполнения, переходы состояния, mining pipeline, сетевой wire-формат, handshake и границы TCP frame. Полноценные многонодовые и reconnect-сценарии пока требуют интеграционных тестов.

## Текущие ограничения и roadmap

- Runtime запускается через обязательный `--env-file`; отдельный пользовательский CLI пока отсутствует.
- Нет загрузки истории блоков, завершённого fork choice и автоматической синхронизации отстающего узла.
- Нет персистентного хранилища: chain, state и mempool находятся в памяти.
- Требуются дополнительные интеграционные тесты TCP framing, handshake, reconnect и распространения блоков.
- Не реализованы discovery, шифрование transport-соединений и защита от всех классов сетевых атак.
- Параметры consensus и формат протокола могут изменяться до стабилизации первой версии.

## Основные зависимости

- `tokio` — асинхронный runtime, TCP и channels;
- `sha2` — SHA-256;
- `ed25519-dalek` — подпись и проверка транзакций;
- `rand_core` и `rand` — генерация ключей и случайных значений;
- `chrono` — timestamps;
- `num-bigint` — расчёт накопленной работы;
- `dotenv` — загрузка runtime-конфигурации узла;
- `hex` — hex-представление данных.

## Contributing

Проект развивается открыто. Для предложений и сообщений об ошибках создавайте issue; изменения можно отправлять через pull request.

# Архітектура Memory MCP Server

## Огляд Високого Рівня
Memory MCP Server - це автономна система пам'яті для AI агентів, написана на Rust. Вона поєднує в собі семантичний пощук (вектори), граф знань та індексацію коду в одному бінарному файлі без зовнішніх залежностей.

### Ключові Компоненти
1. **MCP Server**: Обробляє запити від клієнтів (IDE, Агенти).
2. **Embedding Architecture**: Генерує вектори локально за допомогою `candle` / `ort`.
3. **Storage Layer**: Вбудована SurrealDB для зберігання векторів, графів та метаданих.
4. **Codebase Engine**: Індексує код з використанням Tree-sitter (в процесі розробки).

## Діаграма Компонентів (C4 Container)

```mermaid
graph TD
    User[AI Agent / IDE]
    
    subgraph "Memory MCP Server"
        Handler[MCP Handler]
        
        subgraph "Logic Layer"
            L_Mem[Memory Logic]
            L_Search[Search Logic]
            L_Graph[Graph Logic]
            L_Code[Code Logic]
        end
        
        subgraph "Embedding Subsystem"
            E_Service[Embedding Service]
            E_Queue[Adaptive Queue]
            E_Worker[Embedding Worker]
            E_Engine["Inference Engine (Candle)"]
            E_Cache["Embedding Store (L1/L2)"]
        end
        
        subgraph "Storage Layer"
            S_Surreal[SurrealDB Access]
        end

        User -- "MCP Tools" --> Handler

        Handler -- "store_memory" --> L_Mem
        Handler -- "recall/search" --> L_Search
        Handler -- "create_relation" --> L_Graph
        Handler -- "index_project" --> L_Code
        
        L_Mem -- "Embed Content" --> E_Service
        L_Search -- "Embed Query" --> E_Service
        
        E_Service --> E_Queue
        E_Queue -- "Batched Requests" --> E_Worker
        E_Worker -- "Check Cache" --> E_Cache
        E_Worker -- "Run Model" --> E_Engine
        
        L_Mem & L_Search & L_Graph & L_Code --> S_Surreal
        
        S_Surreal -.-> DB[(Embedded Files)]
        E_Cache -.-> DB
    end
```

## Деталізація Компонентів та Алгоритми

### 1. Logic Layer (Логічний Шар)
Відповідає за обробку запитів, маршрутизацію та реалізацію бізнес-логіки.

*   **Reciprocal Rank Fusion (RRF)**: Алгоритм об'єднання результатів пошуку з різних джерел (Vector Search, BM25, Knowledge Graph).
    *   *Навіщо*: Векторний пошук гарний для семантики ("сенс"), BM25 для точних співпадінь ключових слів, а Граф для зв'язків. RRF дозволяє взяти найкраще з усіх трьох світів без складного тюнінгу ваг.
    *   *Formula*: `score = 1.0 / (k + rank)`
*   **BM25**: Алгоритм текстового пошуку (Okapi BM25). Реалізований поверх індексів SurrealDB.

### 2. Embedding Subsystem (Підсистема Векторизації)
Критичний компонент для семантичного пошуку. Працює автономно.

*   **Adaptive Queue**: Розумна черга, що регулює швидкість обробки запитів на векторизацію (Backpressure).
    *   *Алгоритм*: Моніторить глибину черги та сповільнює нові запити (`THROTTLE_DELAY_MS`), якщо черга заповнена > 80% (`HIGH_WATERMARK`).
    *   *Навіщо*: Запобігає OOM (Out of Memory) при масовій індексації файлів.
*   **Inference Engine (Candle)**: Використовує бібліотеку `candle` (Hugginface) для запуску BERT-подібних моделей (nomic-embed, e5) на CPU. Не потребує Python.
*   **L1/L2 Cache**:
    *   L1: LRU Cache в RAM для найчастіших запитів.
    *   L2: Дисковий кеш (Sled/SurrealDB) для уникнення повторної векторизації незміненого контенту.

### 3. Graph Algorithms (Графові Алгоритми)
Використовуються для аналізу зв'язків між сутностями (файли, функції, нотатки).

*   **Personalized PageRank (PPR)**: Алгоритм ранжування вузлів графа відносно "стартових" точок (seed nodes).
    *   *Застосування*: Коли користувач шукає "Authorization", ми знаходимо вузол "Authorization", а PPR знаходить всі пов'язані концепції (наприклад, "Login", "JWT", "OAuth"), навіть якщо в тексті немає слова "Authorization".
    *   *Hub Dampening*: Модифікація для зменшення ваги "супер-вузлів" (які пов'язані з усім), щоб уникнути шуму.
*   **Leiden Algorithm**: Алгоритм виявлення спільнот (Community Detection).
    *   *Навіщо*: Групує тісно пов'язані файли або концепції в кластери. Допомагає зрозуміти модульну структуру проекту.

### 4. Codebase Engine (Рушій Кодової Бази)
Відповідає за розуміння коду.

*   **Tree-Sitter Chunking**: Розумне розбиття коду на фрагменти (чанки) на основі абстрактного синтаксичного дерева (AST), а не просто по рядках.
    *   *Логіка*: Враховує межі функцій та класів. Великі функції розбиваються на менші логічні блоки, зберігаючи контекст.
    *   *Навіщо*: Векторний пошук працює краще з логічно завершеними шматками коду, ніж з довільними нарізками тексту.
*   **Content Hashing (Blake3)**: Швидке хешування для дедуплікації. Якщо файл не змінився, він не переіндексується.

## Потік Даних: Збереження Пам'яті (Store Memory)

```mermaid
sequenceDiagram
    participant Agent
    participant MCP as MCP Server
    participant Embed as Embedding Service
    participant DB as SurrealDB

    Agent->>MCP: store_memory(content: "...")
    MCP->>Embed: embed(content)
    Embed-->>MCP: [0.12, -0.45, ...] (Vector)
    MCP->>DB: CREATE memory SET content=..., embedding=...
    DB-->>MCP: Memory ID
    MCP-->>Agent: Memory ID
```

## Потік Даних: Пошук (Recall / Hybrid Search)

```mermaid
sequenceDiagram
    participant Agent
    participant MCP as MCP Server
    participant Embed as Embedding Service
    participant DB as SurrealDB

    Agent->>MCP: recall(query: "...")
    par Vector Search
        MCP->>Embed: embed(query)
        Embed-->>MCP: Vector
        MCP->>DB: SELECT * FROM memory WHERE embedding <|5|> vector
    and Stats / Graph Search
        MCP->>DB: SELECT * FROM memory WHERE content CONTAINS query
    end
    DB-->>MCP: Results
    MCP->>MCP: Re-rank (RRF)
    MCP-->>Agent: Top Results
```

## Структура Модулів (Crate Structure)

* `src/main.rs`: Точка входу, ініціалізація CLI та сервісів.
* `src/server/`: Реалізація MCP протоколу та маршрутизація інструментів.
* `src/embedding/`: Обгортка над `candle` для локального інференсу моделей.
* `src/storage/`: Абстракція над SurrealDB.
* `src/graph/`: Алгоритми графів (PageRank, Community Detection).
* `src/codebase/`: Логіка індексації файлів та чанкування.

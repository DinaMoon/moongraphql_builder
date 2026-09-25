# MoonGraphQL Builder 🌙

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/moongraphql_builder.svg?style=flat-square)](https://crates.io/crates/moongraphql_builder)
[![Documentation](https://docs.rs/moongraphql_builder/badge.svg?style=flat-square)](https://docs.rs/moongraphql_builder)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg?style=flat-square)](https://www.rust-lang.org)

[English](README.md) | **Русский**

Высокопроизводительный, строго типизированный фреймворк построения GraphQL-запросов и селекторов полей, спроектированный для критически важных микросервисов и надежных клиентских библиотек.

</div>

---

## 📑 Оглавление

- [Обзор](#-обзор)
- [Ключевые возможности](#-ключевые-возможности)
- [Установка](#-установка)
- [Быстрый старт](#-быстрый-старт)
- [Ключевые концепции](#-ключевые-концепции)
  - [1. Type-State селекторы времени компиляции (`define_selector!`)](#1-type-state-селекторы-времени-компиляции-define_selector)
  - [2. Превентивный расчёт метрик AST (Сложность и Глубина)](#2-превентивный-расчёт-метрик-ast-сложность-и-глубина)
  - [3. Декларативные билдеры и гибридная валидация (`define_query_builder!`)](#3-декларативные-билдеры-и-гибридная-валидация-define_query_builder)
  - [4. Два режима компиляции (Inline vs. GraphQL Variables)](#4-два-режима-компиляции-inline-vs-graphql-variables)
  - [5. Композитные составные запросы (`define_composite_query!`)](#5-композитные-составные-запросы-define_composite_query)
  - [6. Интернационализация через Project Fluent (`i18n`)](#6-интернационализация-через-project-fluent-i18n)
- [Архитектура и реэкспорты](#-архитектура-и-реэкспорты)
- [Лицензия](#-лицензия)

---

## 🌟 Обзор

Использование сырых текстовых строк при формировании GraphQL-запросов в production-системах неизбежно ведет к скрытым ошибкам: случайному дублированию запрашиваемых полей, опечаткам в именах переменных, пропуску обязательных аргументов и падениям с HTTP `422 Unprocessable Entity` или `429 Too Many Requests`, когда итоговый запрос превышает лимиты сложности схемы.

`moongraphql_builder` искореняет эти проблемы на фундаментальном уровне. Сочетая паттерн Type-State времени компиляции, декларативное метапрограммирование через макросы и систему интернационализации Mozilla Project Fluent, библиотека позволяет собирать математически верифицированные, дедуплицированные и проверенные на лимиты GraphQL-документы прямо в оперативной памяти до отправки в сеть.

---

## ⚡ Ключевые возможности

- 🔒 **Type-State на уровне компилятора:** Физический запрет дублирования полей в рамках одного узла при помощи статического анализа Rust и булевых констант с нулевой стоимостью в рантайме.
- 📐 **Детерминированный канонический AST:** Автоматическая дедупликация и алфавитная сортировка полей через `BTreeSet`, гарантирующие воспроизводимость хэшей запросов и максимальную эффективность кэширования HTTP-прокси.
- 📊 **Превентивный расчёт метрик AST:** Рекурсивный расчёт теоретической **Сложности** (Complexity) и максимальной **Глубины** (Depth) графа защищает от блокировок API до совершения сетевого вызова.
- 🛡️ **Гибридная декларативная валидация:** Проверка параметров запроса на уровне атрибутов макроса (`#[validate(required)]`, `range`, `min`, `max`, `min_len`, `max_len`, а также кастомные предикаты) с контекстными метаданными `ArgMeta`.
- 🔄 **Двойной движок компиляции:** Возможность генерации как изолированного инлайн-документа, так и полноценного набора из параметризованного GraphQL-запроса и строго типизированного JSON-словаря переменных.
- 🧩 **Координаторы составных запросов:** Декларативное объединение независимых запросов в единый корневой GraphQL-документ за 1 сетевой RTT.
- 🌍 **Нативная локализация через Project Fluent:** Встроенные двуязычные (`ru` / `en`) человекопонятные сообщения об ошибках валидации с изоляцией контекста на уровне Tokio Task-Local.

---

## 📦 Установка

Добавьте `moongraphql_builder` в ваш `Cargo.toml`:

```toml
[dependencies]
moongraphql_builder = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## 🚀 Быстрый старт

Минимальный пример, демонстрирующий объявление селекторов, сборку запроса, расчёт метрик AST и превентивную клиентскую валидацию:

```rust
use moongraphql_builder::prelude::*;

// 1. Объявляем типизированный селектор полей
define_selector! {
    #[default_complexity = 1]
    #[default = "id name"]
    pub struct AnimeSelector {
        #[complexity = 1]
        id: id,
        #[complexity = 1]
        name: name,
        #[complexity = 1]
        russian: russian,
        #[complexity = 1]
        score: score,
    }
}

// 2. Объявляем билдер запроса с правилами валидации
define_query_builder! {
    #[root = animes]
    #[selector = AnimeSelector]
    pub struct AnimesQueryBuilder {
        #[validate(required)]
        #[validate(min = 1)]
        page: u32 => page("PositiveInt", scalar),

        #[validate(range = 1..=50)]
        limit: u32 => limit("PositiveInt", scalar),

        #[validate(min_len = 2)]
        search: String => search("String", string),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 3. Собираем запрос через лаконичный Fluent API
    let query = AnimesQueryBuilder::new()
        .page(1)
        .limit(10)
        .search("Monogatari")
        .select(|f| f.id().name().russian().score());

    // 4. Превентивная проверка расчетных метрик
    println!("Сложность запроса (Complexity): {}", query.complexity()); // 5 (1 корень + 4 поля)
    println!("Глубина графа    (Depth):      {}", query.depth());      // 1 уровень

    // 5. Превентивная локальная валидация лимитов и аргументов
    let validator = MetricValidator::new(190, 5);
    validator.validate(&query)?;

    // 6. Компиляция в чистый инлайн-документ GraphQL
    let raw_query = query._build_query();
    println!("Скомпилированный запрос:\n{}", raw_query);

    Ok(())
}
```

---

## 🧠 Ключевые концепции

### 1. Type-State селекторы времени компиляции (`define_selector!`)

Обычные билдеры запросов позволяют разработчику по ошибке вызвать `.id().id().name()`, что засоряет полезную нагрузку дубликатами. В `moongraphql_builder` уникальность полей защищена системой типов:

```rust
define_selector! {
    #[default_complexity = 5]
    #[default = "originalUrl mainUrl"]
    pub struct PosterSelector {
        #[complexity = 1]
        id: id,
        #[complexity = 1]
        original_url: originalUrl,
        #[complexity = 1]
        main_url: mainUrl,
    }
}
```

- **Автовывод типов замыканий:** Во вложенных замыканиях вроде `.poster(|p| p.original_url())` тип параметра `p` автоматически выводится компилятором Rust.
- **Поддержка тождественного замыкания:** Вызов `.poster(|p| p)` раскрывает дефолтный набор полей (`originalUrl mainUrl`).
- **Ошибки на этапе компиляции:** Повторный вызов `.id().id()` на одном узле приведёт к ошибке сборки, предотвращая баги до запуска тестов.

### 2. Превентивный расчёт метрик AST (Сложность и Глубина)

Бэкенды на Apollo Server или GraphQL Ruby жестко контролируют очки сложности (Query Complexity) и максимальную вложенность (Query Depth). Превышение лимитов приводит к бесполезной трате трафика и ошибкам HTTP 4xx.

`moongraphql_builder` вычисляет метрики **локально в оперативной памяти**:

**Итоговая сложность** = `Базовый вес корня` + `∑(Веса полей)` + `∑(Веса вложенных объектов)`

```rust
let query = AnimesQueryBuilder::new()
    .page(1)
    .with_max_complexity(250) // Локальное переопределение лимита сложности
    .with_max_depth(5)        // Локальное переопределение лимита глубины
    .select(|f| f.id().name().poster(|p| p.original_url()));

assert_eq!(query.complexity(), 1 + 1 + 1 + 5 + 1); // 9 очков
assert_eq!(query.depth(), 2);                      // 2 уровня вложенности
```

### 3. Декларативные билдеры и гибридная валидация (`define_query_builder!`)

Задавайте превентивные правила валидации аргументов прямо в макросе:

```rust
fn validate_not_blank(val: &String, meta: &ArgMeta) -> Result<(), String> {
    if val.trim().is_empty() {
        return Err(format!("Аргумент '{}' не может состоять только из пробелов", meta.arg_name));
    }
    Ok(())
}

define_query_builder! {
    #[root = animes]
    #[selector = AnimeSelector]
    pub struct AnimesQueryBuilder {
        #[validate(required)]
        page: u32 => page("PositiveInt", scalar),

        #[validate(range = 1..=50)]
        limit: u32 => limit("PositiveInt", scalar),

        #[validate(custom = validate_not_blank)]
        search: String => search("String", string),
    }
}
```

Поддерживаемые атрибуты валидации:
- `#[validate(required)]` — Аргумент обязателен к заполнению.
- `#[validate(min = N)]` / `#[validate(max = N)]` — Нижняя и верхняя числовые границы.
- `#[validate(range = A..=B)]` — Попадание числа в указанный диапазон `RangeInclusive`.
- `#[validate(min_len = N)]` / `#[validate(max_len = N)]` — Проверка длины строк или срезов.
- `#[validate(custom = path)]` — Пользовательская функция проверки с доступом к контексту `ArgMeta`.

### 4. Два режима компиляции (Inline vs. GraphQL Variables)

В зависимости от требований вашего бэкенда компилируйте запросы в подходящий формат:

#### А. Инлайн-документ (Inline Query)
```rust
let inline_doc = query._build_query();
// query { animes(page: 1, limit: 10) { id name } }
```

#### Б. Параметризованный документ с GraphQL Variables
```rust
let (body, decls, vars) = query._build_query_parts(true);
let full_doc = format!("query ({}) {{ {} }}", decls.join(", "), body);

// Document: query ($animes_page: PositiveInt, $animes_limit: PositiveInt) { animes(page: $animes_page, limit: $animes_limit) { id name } }
// Variables JSON: {"animes_page": 1, "animes_limit": 10}
```

### 5. Композитные составные запросы (`define_composite_query!`)

Объединяйте независимые подзапросы в единый сетевой пакет без необходимости отправлять несколько HTTP-запросов:

```rust
define_composite_query! {
    pub struct CatalogCompositeQuery {
        animes: AnimesQueryBuilder,
        mangas: MangasQueryBuilder,
    }
}

let mut composite = CatalogCompositeQuery::new();
composite.animes(|q| q.page(1).limit(5).select(|f| f.id().name()));
composite.mangas(|q| q.page(1).search("Berserk").select(|f| f.id().name()));

let (merged_query, merged_vars) = composite.build_payload_parts();
```

- Автоматическое добавление префиксов предотвращает коллизии имен переменных.
- Кумулятивный расчёт сложности оценивает лимиты всего сводного документа целиком.

### 6. Интернационализация через Project Fluent (`i18n`)

Все диагностические сообщения об ошибках локализованы при помощи **Mozilla Project Fluent**:

```rust
use moongraphql_builder::prelude::*;

// Переключение глобальной локали
set_global_locale("ru");
// Ошибка: "❌ Превышен лимит сложности GraphQL-запроса: рассчитано 250, максимально допустимо 190"

set_global_locale("en");
// Ошибка: "❌ GraphQL query complexity limit exceeded: calculated 250, max allowed 190"
```

Внешние крейты могут подключать свои собственные словари переводов через функцию `register_resource()`.

---

## 🏗️ Архитектура и реэкспорты

Для удобства подключения без бойлерплейта крейт поставляет модульный прелюд:

```rust
use moongraphql_builder::prelude::*;
```

Прелюд автоматически экспортирует:
- Макросы: `define_selector!`, `define_query_builder!`, `define_composite_query!`, `tr!`.
- Базовые трейты: `BuildableSelector`, `GqlInspectable`, `GqlValidatable`, `BuildableQuery`, `CompositeQueryDocument`.
- Модели и валидаторы: `MetricValidator`, `GraphQLPayload`, `GraphQLResponse`, `MoongqlError`, `ArgMeta`.

---

## 📄 Лицензия

Распространяется под двойной свободной лицензией на ваш выбор:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) или <http://www.apache.org/licenses/LICENSE-2.0>)
- **MIT license** ([LICENSE-MIT](LICENSE-MIT) или <http://opensource.org/licenses/MIT>)

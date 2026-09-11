# --- GraphQL Metric Errors ---
err-gql-complexity-exceeded = ❌ Превышен лимит сложности GraphQL-запроса: рассчитано {$calculated}, максимально допустимо {$limit}
err-gql-depth-exceeded = ❌ Превышен лимит глубины GraphQL-запроса: рассчитана глубина {$calculated}, максимально допустимо {$limit}
err-gql-validation = ❌ Ошибка валидации GraphQL: {$error}

# --- GraphQL Argument Validation Errors ---
err-val-range = Аргумент '{$arg}' в запросе '{$query}' должен быть в диапазоне {$range} (получено: {$got})
err-val-min = Аргумент '{$arg}' в запросе '{$query}' должен быть не менее {$min} (получено: {$got})
err-val-max = Аргумент '{$arg}' в запросе '{$query}' должен быть не более {$max} (получено: {$got})
err-val-min-len = Длина аргумента '{$arg}' в запросе '{$query}' должна быть не менее {$len} (получено: {$got})
err-val-max-len = Длина аргумента '{$arg}' в запросе '{$query}' должна быть не более {$len} (получено: {$got})
err-val-required = Обязательный аргумент '{$arg}' не указан в запросе '{$query}'

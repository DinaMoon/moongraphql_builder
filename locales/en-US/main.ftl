# --- GraphQL Metric Errors ---
err-gql-complexity-exceeded = ❌ GraphQL query complexity limit exceeded: calculated {$calculated}, max allowed {$limit}
err-gql-depth-exceeded = ❌ GraphQL query depth limit exceeded: calculated {$calculated}, max allowed {$limit}
err-gql-validation = ❌ GraphQL validation error: {$error}

# --- GraphQL Argument Validation Errors ---
err-val-range = Argument '{$arg}' in query '{$query}' must be in range {$range} (got: {$got})
err-val-min = Argument '{$arg}' in query '{$query}' must be at least {$min} (got: {$got})
err-val-max = Argument '{$arg}' in query '{$query}' must be at most {$max} (got: {$got})
err-val-min-len = Argument '{$arg}' in query '{$query}' length must be at least {$len} (got: {$got})
err-val-max-len = Argument '{$arg}' in query '{$query}' length must be at most {$len} (got: {$got})
err-val-required = Required argument '{$arg}' is missing in query '{$query}'

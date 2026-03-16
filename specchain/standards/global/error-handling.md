# Error Handling

## General Principles
- Fail fast, fail loudly in development
- Handle errors gracefully in production
- Log errors with context

## Patterns
- Use try/catch for async operations
- Return Result types for expected failures
- Throw errors for unexpected failures

## User-Facing Errors
- Show friendly error messages
- Provide actionable guidance
- Never expose internal details

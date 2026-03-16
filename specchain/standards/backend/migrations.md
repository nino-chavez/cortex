# Migration Standards

## Naming
- Descriptive migration names
- Include timestamp prefix
- One change per migration

## Safety
- Always write reversible migrations
- Test rollbacks
- Backup before production migrations

## Performance
- Add indexes for foreign keys
- Consider table size for ALTER operations
- Use batching for data migrations

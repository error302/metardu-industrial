## Pull Request

### Summary

<!-- One paragraph max. What does this PR add, fix, or change? -->

### Type

- [ ] feat — new feature
- [ ] fix — bug fix
- [ ] refactor — no behavior change
- [ ] perf — performance improvement
- [ ] docs — documentation only
- [ ] ci — build/CI changes
- [ ] test — test additions
- [ ] chore — misc

### Domain

- [ ] Mining
- [ ] Marine
- [ ] Both
- [ ] Cross-cutting

### Checklist

- [ ] `npm run build` passes locally
- [ ] `cargo fmt --all -- --check` passes locally
- [ ] `cargo clippy --all-targets -- -D warnings` passes locally
- [ ] Commit messages follow Conventional Commits
- [ ] No new design tokens hardcoded — used existing `src/lib/tokens.ts`
- [ ] New IPC commands have typed wrappers in `src/lib/tauri-ipc.ts`
- [ ] Documentation updated (README, ARCHITECTURE, or inline) if behavior changed

### Testing

<!-- How did you verify this works? -->

### Screenshots / recordings

<!-- For UI changes only -->

### Related issues

<!-- "Closes #123" / "Refs #456" -->

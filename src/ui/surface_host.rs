#[cfg(test)]
mod tests {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestAppSurfaceRef {
        LegacyCenterContent,
        TerminalWorkspace,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestStageState {
        active: TestAppSurfaceRef,
    }

    #[test]
    fn stage_starts_on_terminal_workspace() {
        let stage = stage_state_for_test();

        assert_eq!(stage.active, TestAppSurfaceRef::TerminalWorkspace);
    }

    fn stage_state_for_test() -> TestStageState {
        // RED-only seam: SF4.1 replaces the legacy center-content marker with
        // production typed Stage state whose default owns TerminalWorkspace.
        TestStageState {
            active: TestAppSurfaceRef::LegacyCenterContent,
        }
    }
}

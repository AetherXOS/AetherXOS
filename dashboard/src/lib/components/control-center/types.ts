export type ControlTab = 'operations' | 'build' | 'config' | 'blueprints' | 'plugins' | 'launcher';
export type ControlPriority = 'high' | 'normal' | 'low';
export type AutoPresetMode = 'balanced' | 'fast_dev' | 'reliable_ci';
export type ComposeGoal = 'boot_min' | 'linux_full' | 'release_hardening';
export type LauncherAction = 'start' | 'stop' | 'restart';
export type OverrideTemplateMode = 'minimal' | 'full';
export type DriftApplyMode = 'full' | 'missing_only';

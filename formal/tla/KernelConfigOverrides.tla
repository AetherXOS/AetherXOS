---- MODULE KernelConfigOverrides ----
EXTENDS Naturals, TLC

VARIABLES telemetryEnabled, historyLen

Init ==
    /\ telemetryEnabled \in BOOLEAN
    /\ historyLen = 1

ToggleTelemetry ==
    /\ telemetryEnabled' = ~telemetryEnabled
    /\ historyLen' = historyLen

GrowHistory ==
    /\ historyLen' = historyLen + 1
    /\ telemetryEnabled' = telemetryEnabled

ResetHistory ==
    /\ historyLen' = 1
    /\ telemetryEnabled' = telemetryEnabled

Next == ToggleTelemetry \/ GrowHistory \/ ResetHistory

HistoryInvariant == historyLen >= 1

====

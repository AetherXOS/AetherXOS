theory Kernel_Config
  imports Main
begin

lemma telemetry_history_len_positive:
  fixes n :: nat
  assumes "n >= 1"
  shows "n > 0"
  using assms by auto

end

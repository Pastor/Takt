// Порождено компилятором Takt (taktc) — цель: SystemVerilog (IEEE 1800).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Такт модели Takt ≡ фронт clk (posedge). Сброс синхронный, активный низкий:
// ветвь if (!rst_n) несёт стартовое состояние — синтетического INIT нет,
// поэтому тело стартового состояния исполняется на такте 1 (контракт 0033).

module batch_cycle (
    input  logic clk,   // служебный порт цели sv: в .takt его нет
    input  logic rst_n, // служебный порт цели sv: сброс, активный низкий
    input  logic en = 1'b1, // служебный порт цели sv: clock enable; НЕ обязателен (умолчание 1)
    output logic ready,
    output logic is_done
);
    // Состояния модели 'Dose (BatchCycle:Dose)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [1:0] {
        BATCH_CYCLE_DOSE_FILL = 2'd0,
        BATCH_CYCLE_DOSE_FULL = 2'd1,
        BATCH_CYCLE_DOSE_END = 2'd2
    } batch_cycle_dose_state_e;

    // Состояния модели 'Drain (BatchCycle:Drain)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [1:0] {
        BATCH_CYCLE_DRAIN_DRY = 2'd0,
        BATCH_CYCLE_DRAIN_EMPTY = 2'd1,
        BATCH_CYCLE_DRAIN_END = 2'd2
    } batch_cycle_drain_state_e;

    // Состояния модели 'Mix (BatchCycle:Mix)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [1:0] {
        BATCH_CYCLE_MIX_BLENDED = 2'd0,
        BATCH_CYCLE_MIX_STIR = 2'd1,
        BATCH_CYCLE_MIX_END = 2'd2
    } batch_cycle_mix_state_e;

    // Состояния модели 'batch_cycle (BatchCycle)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [1:0] {
        BATCH_CYCLE_CYCLE = 2'd0,
        BATCH_CYCLE_DONE = 2'd1,
        BATCH_CYCLE_END = 2'd2
    } batch_cycle_state_e;

    // Шаг последовательной композиции 'Cycle (BatchCycle:Cycle)' (`+`).
    typedef enum logic [1:0] {
        BATCH_CYCLE_CYCLE_STEP_0 = 2'd0,
        BATCH_CYCLE_CYCLE_STEP_1 = 2'd1,
        BATCH_CYCLE_CYCLE_STEP_2 = 2'd2
    } batch_cycle_cycle_step_e;

    batch_cycle_dose_state_e batch_cycle_dose_state;
    batch_cycle_dose_state_e batch_cycle_dose_state_next;
    logic [7:0] batch_cycle_dose_dosed;
    logic [7:0] batch_cycle_dose_dosed_next;
    batch_cycle_drain_state_e batch_cycle_drain_state;
    batch_cycle_drain_state_e batch_cycle_drain_state_next;
    logic [7:0] batch_cycle_drain_drained;
    logic [7:0] batch_cycle_drain_drained_next;
    batch_cycle_mix_state_e batch_cycle_mix_state;
    batch_cycle_mix_state_e batch_cycle_mix_state_next;
    logic [7:0] batch_cycle_mix_stirred;
    logic [7:0] batch_cycle_mix_stirred_next;
    batch_cycle_state_e state;
    batch_cycle_state_e state_next;
    logic [7:0] batch_cycle_stage;
    logic [7:0] batch_cycle_stage_next;
    batch_cycle_cycle_step_e batch_cycle_cycle_step;
    batch_cycle_cycle_step_e batch_cycle_cycle_step_next;
    logic ready_next;

    // Комбинационная часть: БЛОКИРУЮЩИЕ присваивания, поэтому порядок
    // операторов и видимость записей внутри такта — в точности как в C.
    always_comb begin
        // Умолчание «остаться как есть». Без него неполное присваивание
        // даёт защёлку (verilator: LATCH).
        batch_cycle_dose_state_next = batch_cycle_dose_state;
        batch_cycle_dose_dosed_next = batch_cycle_dose_dosed;
        batch_cycle_drain_state_next = batch_cycle_drain_state;
        batch_cycle_drain_drained_next = batch_cycle_drain_drained;
        batch_cycle_mix_state_next = batch_cycle_mix_state;
        batch_cycle_mix_stirred_next = batch_cycle_mix_stirred;
        state_next = state;
        batch_cycle_stage_next = batch_cycle_stage;
        batch_cycle_cycle_step_next = batch_cycle_cycle_step;
        ready_next = ready;

        unique case (state)
            BATCH_CYCLE_CYCLE: begin
                unique case (batch_cycle_cycle_step)
                    BATCH_CYCLE_CYCLE_STEP_0: begin
                        // Под-модель 'Dose (BatchCycle:Dose)' — инлайн её такта.
                        unique case (batch_cycle_dose_state)
                            BATCH_CYCLE_DOSE_FILL: begin
                                batch_cycle_stage_next = 1;
                                batch_cycle_dose_dosed_next = (batch_cycle_dose_dosed_next + 1);
                                if ((batch_cycle_dose_dosed_next >= 3)) begin
                                    batch_cycle_dose_state_next = BATCH_CYCLE_DOSE_FULL;
                                end
                            end
                            BATCH_CYCLE_DOSE_FULL: begin
                                batch_cycle_dose_state_next = BATCH_CYCLE_DOSE_END;
                            end
                            BATCH_CYCLE_DOSE_END: begin end
                        endcase
                        if ((batch_cycle_dose_state_next == BATCH_CYCLE_DOSE_END)) begin
                            batch_cycle_cycle_step_next = BATCH_CYCLE_CYCLE_STEP_1;
                        end
                    end
                    BATCH_CYCLE_CYCLE_STEP_1: begin
                        // Под-модель 'Mix (BatchCycle:Mix)' — инлайн её такта.
                        unique case (batch_cycle_mix_state)
                            BATCH_CYCLE_MIX_BLENDED: begin
                                batch_cycle_mix_state_next = BATCH_CYCLE_MIX_END;
                            end
                            BATCH_CYCLE_MIX_STIR: begin
                                batch_cycle_stage_next = 2;
                                batch_cycle_mix_stirred_next = (batch_cycle_mix_stirred_next + 1);
                                if ((batch_cycle_mix_stirred_next >= 2)) begin
                                    batch_cycle_mix_state_next = BATCH_CYCLE_MIX_BLENDED;
                                end
                            end
                            BATCH_CYCLE_MIX_END: begin end
                        endcase
                        if ((batch_cycle_mix_state_next == BATCH_CYCLE_MIX_END)) begin
                            batch_cycle_cycle_step_next = BATCH_CYCLE_CYCLE_STEP_2;
                        end
                    end
                    BATCH_CYCLE_CYCLE_STEP_2: begin
                        // Под-модель 'Drain (BatchCycle:Drain)' — инлайн её такта.
                        unique case (batch_cycle_drain_state)
                            BATCH_CYCLE_DRAIN_DRY: begin
                                batch_cycle_drain_state_next = BATCH_CYCLE_DRAIN_END;
                            end
                            BATCH_CYCLE_DRAIN_EMPTY: begin
                                batch_cycle_stage_next = 3;
                                batch_cycle_drain_drained_next = (batch_cycle_drain_drained_next + 1);
                                if ((batch_cycle_drain_drained_next >= 2)) begin
                                    batch_cycle_drain_state_next = BATCH_CYCLE_DRAIN_DRY;
                                end
                            end
                            BATCH_CYCLE_DRAIN_END: begin end
                        endcase
                        if ((batch_cycle_drain_state_next == BATCH_CYCLE_DRAIN_END)) begin
                            state_next = BATCH_CYCLE_DONE;
                        end
                    end
                endcase
            end
            BATCH_CYCLE_DONE: begin
                ready_next = 1;
            end
            BATCH_CYCLE_END: begin end
        endcase
    end

    // Регистровая часть: НЕБЛОКИРУЮЩИЕ присваивания. Ветвь сброса несёт
    // стартовые состояния ВСЕХ уровней — они сбрасываются одним фронтом,
    // поэтому сдвиг такта равен нулю на любой глубине (контракт 0033).
    always_ff @(posedge clk) begin
        if (!rst_n) begin
            batch_cycle_dose_state <= BATCH_CYCLE_DOSE_FILL;
            batch_cycle_dose_dosed <= 0;
            batch_cycle_drain_state <= BATCH_CYCLE_DRAIN_EMPTY;
            batch_cycle_drain_drained <= 0;
            batch_cycle_mix_state <= BATCH_CYCLE_MIX_STIR;
            batch_cycle_mix_stirred <= 0;
            state <= BATCH_CYCLE_CYCLE;
            batch_cycle_stage <= 0;
            batch_cycle_cycle_step <= BATCH_CYCLE_CYCLE_STEP_0;
            ready <= '0;
        end else if (en) begin
            batch_cycle_dose_state <= batch_cycle_dose_state_next;
            batch_cycle_dose_dosed <= batch_cycle_dose_dosed_next;
            batch_cycle_drain_state <= batch_cycle_drain_state_next;
            batch_cycle_drain_drained <= batch_cycle_drain_drained_next;
            batch_cycle_mix_state <= batch_cycle_mix_state_next;
            batch_cycle_mix_stirred <= batch_cycle_mix_stirred_next;
            state <= state_next;
            batch_cycle_stage <= batch_cycle_stage_next;
            batch_cycle_cycle_step <= batch_cycle_cycle_step_next;
            ready <= ready_next;
        end
    end

    // Терминальность модели наблюдаема снаружи — аналог _is_done() цели c.
    assign is_done = (state == BATCH_CYCLE_END);
endmodule

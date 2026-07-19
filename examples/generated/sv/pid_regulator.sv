// Порождено компилятором Lam (lamc) — цель: SystemVerilog (IEEE 1800).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Такт модели Lam ≡ фронт clk (posedge). Сброс синхронный, активный низкий:
// ветвь if (!rst_n) несёт стартовое состояние — синтетического INIT нет,
// поэтому тело стартового состояния исполняется на такте 1 (контракт 0033).

module pid_regulator (
    input  logic clk,   // служебный порт цели sv: в .lam его нет
    input  logic rst_n, // служебный порт цели sv: сброс, активный низкий
    output logic ready,
    output logic is_done
);
    // Состояния модели 'Pid (PidRegulator:Pid)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [1:0] {
        PID_REGULATOR_PID_CONTROL = 2'd0,
        PID_REGULATOR_PID_DONE = 2'd1,
        PID_REGULATOR_PID_SETTLED = 2'd2,
        PID_REGULATOR_PID_END = 2'd3
    } pid_regulator_pid_state_e;

    // Состояния модели 'pid_regulator (PidRegulator)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [0:0] {
        PID_REGULATOR_MAIN = 1'd0,
        PID_REGULATOR_END = 1'd1
    } pid_regulator_state_e;

    pid_regulator_pid_state_e pid_regulator_pid_state;
    pid_regulator_pid_state_e pid_regulator_pid_state_next;
    logic signed [15:0] pid_regulator_pid_ctrl;
    logic signed [15:0] pid_regulator_pid_ctrl_next;
    logic signed [15:0] pid_regulator_pid_deriv;
    logic signed [15:0] pid_regulator_pid_deriv_next;
    logic signed [15:0] pid_regulator_pid_eps;
    logic signed [15:0] pid_regulator_pid_eps_next;
    logic signed [15:0] pid_regulator_pid_err;
    logic signed [15:0] pid_regulator_pid_err_next;
    logic signed [15:0] pid_regulator_pid_err_prev;
    logic signed [15:0] pid_regulator_pid_err_prev_next;
    logic signed [15:0] pid_regulator_pid_i_acc;
    logic signed [15:0] pid_regulator_pid_i_acc_next;
    logic signed [15:0] pid_regulator_pid_imax;
    logic signed [15:0] pid_regulator_pid_imax_next;
    logic signed [15:0] pid_regulator_pid_kd;
    logic signed [15:0] pid_regulator_pid_kd_next;
    logic signed [15:0] pid_regulator_pid_ki;
    logic signed [15:0] pid_regulator_pid_ki_next;
    logic signed [15:0] pid_regulator_pid_kp;
    logic signed [15:0] pid_regulator_pid_kp_next;
    logic signed [15:0] pid_regulator_pid_kplant;
    logic signed [15:0] pid_regulator_pid_kplant_next;
    logic signed [15:0] pid_regulator_pid_meas;
    logic signed [15:0] pid_regulator_pid_meas_next;
    logic signed [15:0] pid_regulator_pid_neg_imax;
    logic signed [15:0] pid_regulator_pid_neg_imax_next;
    logic signed [15:0] pid_regulator_pid_target;
    logic signed [15:0] pid_regulator_pid_target_next;
    pid_regulator_state_e state;
    pid_regulator_state_e state_next;
    logic ready_next;

    // Комбинационная часть: БЛОКИРУЮЩИЕ присваивания, поэтому порядок
    // операторов и видимость записей внутри такта — в точности как в C.
    always_comb begin
        // Умолчание «остаться как есть». Без него неполное присваивание
        // даёт защёлку (verilator: LATCH).
        pid_regulator_pid_state_next = pid_regulator_pid_state;
        pid_regulator_pid_ctrl_next = pid_regulator_pid_ctrl;
        pid_regulator_pid_deriv_next = pid_regulator_pid_deriv;
        pid_regulator_pid_eps_next = pid_regulator_pid_eps;
        pid_regulator_pid_err_next = pid_regulator_pid_err;
        pid_regulator_pid_err_prev_next = pid_regulator_pid_err_prev;
        pid_regulator_pid_i_acc_next = pid_regulator_pid_i_acc;
        pid_regulator_pid_imax_next = pid_regulator_pid_imax;
        pid_regulator_pid_kd_next = pid_regulator_pid_kd;
        pid_regulator_pid_ki_next = pid_regulator_pid_ki;
        pid_regulator_pid_kp_next = pid_regulator_pid_kp;
        pid_regulator_pid_kplant_next = pid_regulator_pid_kplant;
        pid_regulator_pid_meas_next = pid_regulator_pid_meas;
        pid_regulator_pid_neg_imax_next = pid_regulator_pid_neg_imax;
        pid_regulator_pid_target_next = pid_regulator_pid_target;
        state_next = state;
        ready_next = ready;

        unique case (state)
            PID_REGULATOR_MAIN: begin
                // Под-модель 'Pid (PidRegulator:Pid)' — инлайн её такта.
                unique case (pid_regulator_pid_state)
                    PID_REGULATOR_PID_CONTROL: begin
                        pid_regulator_pid_err_next = (16'($signed(pid_regulator_pid_target_next) - $signed(pid_regulator_pid_meas_next)));
                        pid_regulator_pid_i_acc_next = (16'($signed(pid_regulator_pid_i_acc_next) + $signed(pid_regulator_pid_err_next)));
                        if ((pid_regulator_pid_i_acc_next > pid_regulator_pid_imax_next)) begin
                            pid_regulator_pid_i_acc_next = pid_regulator_pid_imax_next;
                        end
                        if ((pid_regulator_pid_i_acc_next < pid_regulator_pid_neg_imax_next)) begin
                            pid_regulator_pid_i_acc_next = pid_regulator_pid_neg_imax_next;
                        end
                        pid_regulator_pid_deriv_next = (16'($signed(pid_regulator_pid_err_next) - $signed(pid_regulator_pid_err_prev_next)));
                        pid_regulator_pid_ctrl_next = (16'($signed((16'($signed((16'(((32'($signed(pid_regulator_pid_kp_next)) * 32'($signed(pid_regulator_pid_err_next))) >>> 8)))) + $signed((16'(((32'($signed(pid_regulator_pid_ki_next)) * 32'($signed(pid_regulator_pid_i_acc_next))) >>> 8))))))) + $signed((16'(((32'($signed(pid_regulator_pid_kd_next)) * 32'($signed(pid_regulator_pid_deriv_next))) >>> 8))))));
                        pid_regulator_pid_meas_next = (16'($signed(pid_regulator_pid_meas_next) + $signed((16'(((32'($signed(pid_regulator_pid_kplant_next)) * 32'($signed(pid_regulator_pid_ctrl_next))) >>> 8))))));
                        pid_regulator_pid_err_prev_next = pid_regulator_pid_err_next;
                        if ((pid_regulator_pid_err_next < pid_regulator_pid_eps_next)) begin
                            pid_regulator_pid_state_next = PID_REGULATOR_PID_SETTLED;
                        end
                    end
                    PID_REGULATOR_PID_DONE: begin
                        ready_next = 1;
                        pid_regulator_pid_state_next = PID_REGULATOR_PID_END;
                    end
                    PID_REGULATOR_PID_SETTLED: begin
                        pid_regulator_pid_meas_next = pid_regulator_pid_target_next;
                        begin
                            pid_regulator_pid_state_next = PID_REGULATOR_PID_DONE;
                        end
                    end
                    PID_REGULATOR_PID_END: begin end
                endcase
                if ((pid_regulator_pid_state_next == PID_REGULATOR_PID_END)) begin
                    state_next = PID_REGULATOR_END;
                end
            end
            PID_REGULATOR_END: begin end
        endcase
    end

    // Регистровая часть: НЕБЛОКИРУЮЩИЕ присваивания. Ветвь сброса несёт
    // стартовые состояния ВСЕХ уровней — они сбрасываются одним фронтом,
    // поэтому сдвиг такта равен нулю на любой глубине (контракт 0033).
    always_ff @(posedge clk) begin
        if (!rst_n) begin
            pid_regulator_pid_state <= PID_REGULATOR_PID_CONTROL;
            pid_regulator_pid_ctrl <= 0;
            pid_regulator_pid_deriv <= 0;
            pid_regulator_pid_eps <= 32;
            pid_regulator_pid_err <= 0;
            pid_regulator_pid_err_prev <= 0;
            pid_regulator_pid_i_acc <= 0;
            pid_regulator_pid_imax <= 8192;
            pid_regulator_pid_kd <= 64;
            pid_regulator_pid_ki <= 16;
            pid_regulator_pid_kp <= 128;
            pid_regulator_pid_kplant <= 128;
            pid_regulator_pid_meas <= 0;
            pid_regulator_pid_neg_imax <= -8192;
            pid_regulator_pid_target <= 2048;
            state <= PID_REGULATOR_MAIN;
            ready <= '0;
        end else begin
            pid_regulator_pid_state <= pid_regulator_pid_state_next;
            pid_regulator_pid_ctrl <= pid_regulator_pid_ctrl_next;
            pid_regulator_pid_deriv <= pid_regulator_pid_deriv_next;
            pid_regulator_pid_eps <= pid_regulator_pid_eps_next;
            pid_regulator_pid_err <= pid_regulator_pid_err_next;
            pid_regulator_pid_err_prev <= pid_regulator_pid_err_prev_next;
            pid_regulator_pid_i_acc <= pid_regulator_pid_i_acc_next;
            pid_regulator_pid_imax <= pid_regulator_pid_imax_next;
            pid_regulator_pid_kd <= pid_regulator_pid_kd_next;
            pid_regulator_pid_ki <= pid_regulator_pid_ki_next;
            pid_regulator_pid_kp <= pid_regulator_pid_kp_next;
            pid_regulator_pid_kplant <= pid_regulator_pid_kplant_next;
            pid_regulator_pid_meas <= pid_regulator_pid_meas_next;
            pid_regulator_pid_neg_imax <= pid_regulator_pid_neg_imax_next;
            pid_regulator_pid_target <= pid_regulator_pid_target_next;
            state <= state_next;
            ready <= ready_next;
        end
    end

    // Терминальность модели наблюдаема снаружи — аналог _is_done() цели c.
    assign is_done = (state == PID_REGULATOR_END);
endmodule

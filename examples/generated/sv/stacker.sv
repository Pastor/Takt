// Порождено компилятором Lam (lamc) — цель: SystemVerilog (IEEE 1800).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Такт модели Lam ≡ фронт clk (posedge). Сброс синхронный, активный низкий:
// ветвь if (!rst_n) несёт стартовое состояние — синтетического INIT нет,
// поэтому тело стартового состояния исполняется на такте 1 (контракт 0033).

module stacker (
    input  logic clk,   // служебный порт цели sv: в .lam его нет
    input  logic rst_n, // служебный порт цели sv: сброс, активный низкий
    output logic is_done
);
    typedef enum logic [0:0] {
        ST_START = 1'd0,
        ST_END   = 1'd1
    } state_e;

    state_e state, state_next;

    always_comb begin
        state_next = state;
    end

    always_ff @(posedge clk) begin
        if (!rst_n) begin
            state <= ST_START;
        end else begin
            state <= state_next;
        end
    end

    assign is_done = (state == ST_END);
endmodule

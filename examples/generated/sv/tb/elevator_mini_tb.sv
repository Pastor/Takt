// Тестбенч цели sv для порождённого elevator_mini.sv (фича 0045).
//
// НЕ порождается lamc: тестбенч — принадлежность ПРОВЕРКИ, а не продукта
// (решение 0045-07). Лежит в подкаталоге tb/, поэтому глоб `*.sv` гейта цели sv
// его не подхватывает.
//
// Задача — та же, что у stacker_tb.sv:
//   1. ПРОВЕРКА ФУНКЦИОНИРОВАНИЯ через assert (провал → $fatal → красный
//      предкоммит).
//   2. ОСЦИЛЛОГРАММА elevator_mini.vcd для gtkwave.
//
// Ручной прогон (из examples/generated/sv/):
//   $ verilator --binary --timing --trace --top-module tb tb/elevator_mini_tb.sv elevator_mini.sv -o simtb
//   (cd tb && ../obj_dir/simtb)   # пишет tb/elevator_mini.vcd
//   gtkwave tb/elevator_mini.vcd
`timescale 1ns / 1ps

module tb;
    // Служебные сигналы цели sv.
    logic clk = 0;
    logic rst_n = 0;

    // Сценарные входы. Нажимаем кнопку кабины «этаж 5»; остальные — 0.
    logic CabinButton_F5 = 0;

    // Датчики концевиков двигателя не используем — двигатель останавливается
    // по команде STOP (приезд на этаж), а не по концевику.
    logic ElevatorMotor_SensorD = 0;
    logic ElevatorMotor_SensorU = 0;

    // Выходы модуля.
    logic DoorOpen, ElevatorMotor_Down, ElevatorMotor_Stop, ElevatorMotor_Up;
    logic is_done;

    // ЗАМКНУТАЯ СРЕДА «ФИЗИКА ЛИФТА». Кабина ползёт к целевому этажу по одному
    // этажу за такт: пока идёт движение и цель не достигнута, «срабатывает»
    // концевик СЛЕДУЮЩЕГО этажа в сторону цели. Внутренние текущий/целевой
    // этаж и состояние читаются иерархической ссылкой (verilator --binary её
    // разрешает).
    wire [7:0] cf = dut.elevator_mini_current_floor;
    wire [7:0] tf = dut.elevator_mini_target_floor;
    wire moving = (dut.elevator_mini_cabin_state == 2'd2); // CABIN_MOVING
    wire [7:0] nf = (tf > cf) ? (cf + 8'd1) : (tf < cf) ? (cf - 8'd1) : cf;
    wire walk = moving && (cf != tf);

    // Одногорячий набор концевиков этажей F1..F9 из «следующего этажа» nf.
    wire FloorSensor_F1_Bottom = walk && (nf == 8'd1);
    wire FloorSensor_F2_Bottom = walk && (nf == 8'd2);
    wire FloorSensor_F3_Bottom = walk && (nf == 8'd3);
    wire FloorSensor_F4_Bottom = walk && (nf == 8'd4);
    wire FloorSensor_F5_Bottom = walk && (nf == 8'd5);
    wire FloorSensor_F6_Bottom = walk && (nf == 8'd6);
    wire FloorSensor_F7_Bottom = walk && (nf == 8'd7);
    wire FloorSensor_F8_Bottom = walk && (nf == 8'd8);
    wire FloorSensor_F9_Bottom = walk && (nf == 8'd9);

    elevator_mini dut (
        .clk(clk),
        .rst_n(rst_n),
        // Неиспользуемые кнопки/датчики тянем в 0 прямо в списке портов.
        .CabinButton_DC(1'b0),
        .CabinButton_F1(1'b0),
        .CabinButton_F2(1'b0),
        .CabinButton_F3(1'b0),
        .CabinButton_F4(1'b0),
        .CabinButton_F5(CabinButton_F5),
        .CabinButton_F6(1'b0),
        .CabinButton_F7(1'b0),
        .CabinButton_F8(1'b0),
        .CabinButton_F9(1'b0),
        .FloorButton_F1(1'b0),
        .FloorButton_F2(1'b0),
        .FloorButton_F3(1'b0),
        .FloorButton_F4(1'b0),
        .FloorButton_F5(1'b0),
        .FloorButton_F6(1'b0),
        .FloorButton_F7(1'b0),
        .FloorButton_F8(1'b0),
        .FloorButton_F9(1'b0),
        .FloorSensor_F1_Bottom(FloorSensor_F1_Bottom),
        .FloorSensor_F2_Bottom(FloorSensor_F2_Bottom),
        .FloorSensor_F3_Bottom(FloorSensor_F3_Bottom),
        .FloorSensor_F4_Bottom(FloorSensor_F4_Bottom),
        .FloorSensor_F5_Bottom(FloorSensor_F5_Bottom),
        .FloorSensor_F6_Bottom(FloorSensor_F6_Bottom),
        .FloorSensor_F7_Bottom(FloorSensor_F7_Bottom),
        .FloorSensor_F8_Bottom(FloorSensor_F8_Bottom),
        .FloorSensor_F9_Bottom(FloorSensor_F9_Bottom),
        .ElevatorMotor_SensorD(ElevatorMotor_SensorD),
        .ElevatorMotor_SensorU(ElevatorMotor_SensorU),
        .DoorOpen(DoorOpen),
        .ElevatorMotor_Down(ElevatorMotor_Down),
        .ElevatorMotor_Stop(ElevatorMotor_Stop),
        .ElevatorMotor_Up(ElevatorMotor_Up),
        .is_done(is_done)
    );

    always #5 clk = ~clk;

    // Накопители событий за прогон (устойчивы к сдвигу такта на один).
    logic saw_up = 0, saw_door = 0, saw_at_floor = 0;
    always @(posedge clk) begin
        if (ElevatorMotor_Up) saw_up <= 1;
        if (DoorOpen) saw_door <= 1;
        // CABIN_AT_FLOOR = 2'd0: кабина доехала до целевого этажа.
        if (dut.elevator_mini_cabin_state == 2'd0) saw_at_floor <= 1;
    end

    integer i;
    initial begin
        $dumpfile("elevator_mini.vcd");
        $dumpvars(0, tb);

        // Первый фронт снимает сброс.
        @(posedge clk);
        rst_n <= 1'b1;

        // Нажимаем кнопку кабины «этаж 5» на несколько тактов (стартуем с
        // этажа 1 — таково значение регистра после сброса).
        CabinButton_F5 <= 1'b1;
        repeat (3) @(posedge clk);
        CabinButton_F5 <= 1'b0;

        // «Физика лифта» довозит кабину до этажа 5; 30 тактов с запасом.
        for (i = 0; i < 30; i = i + 1) @(posedge clk);

        // Проверка функционирования: двигатель поехал вверх (ElevatorMotor_Up),
        // дверь открывалась (DoorOpen) и кабина доехала до этажа (AT_FLOOR).
        if (!saw_up)       $error("elevator: ElevatorMotor_Up не поднялся — кабина не поехала вверх");
        if (!saw_door)     $error("elevator: DoorOpen не поднялся — дверь не открывалась");
        if (!saw_at_floor) $error("elevator: кабина не доехала до целевого этажа (AT_FLOOR)");

        if (saw_up && saw_door && saw_at_floor)
            $display("elevator_mini_tb: OK (наблюдались ElevatorMotor_Up, DoorOpen, приезд на этаж)");
        else
            $fatal(1, "elevator_mini_tb: ПРОВАЛ функциональной проверки");

        $finish;
    end
endmodule

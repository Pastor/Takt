(* П3. Полная обёртка: CONFIGURATION / RESOURCE / TASK / PROGRAM.
   Форма, которую генератор обязан эмитить, если П2 красная. *)
FUNCTION_BLOCK Demo
VAR
    state : USINT := 0;
    count : USINT := 0;
END_VAR
    CASE state OF
        0:
            count := count + 1;
        1:
            count := 0;
    END_CASE;
END_FUNCTION_BLOCK

PROGRAM Main
VAR
    inst : Demo;
END_VAR
    inst();
END_PROGRAM

CONFIGURATION Config0
    RESOURCE Res0 ON PLC
        TASK Tick(INTERVAL := T#100ms, PRIORITY := 0);
        PROGRAM Inst0 WITH Tick : Main;
    END_RESOURCE
END_CONFIGURATION

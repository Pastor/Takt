(* П2. Голый FUNCTION_BLOCK без CONFIGURATION.
   Ключевая проба: принимает ли iec2c отдельный блок, или требует полной
   программы для ПЛК. От исхода зависит, обязан ли генератор эмитить
   CONFIGURATION (флаг --st-configuration). *)
FUNCTION_BLOCK Demo
VAR
    state : USINT := 0;
    count : USINT := 0;
END_VAR
    CASE state OF
        0:
            count := count + 1;
            IF count >= 3 THEN
                state := 1;
            END_IF;
        1:
            count := 0;
    END_CASE;
END_FUNCTION_BLOCK

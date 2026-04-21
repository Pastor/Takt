/**
 * elevator_mini_main.c — Эмулятор датчиков и проверка выходов 9-этажного лифта.
 *
 * Использование:
 *   ./elevator_mini <scenario.txt> <output.txt>
 *
 * Формат входного файла (scenario.txt):
 *   - Каждая непустая строка = один такт симуляции
 *   - Строки, начинающиеся с '#', — комментарии (пропускаются)
 *   - Формат такта: PORT_NAME=0|1 [PORT_NAME=0|1 ...] (пробел-разделители)
 *   - Все неупомянутые входные порты считаются равными 0 в этом такте
 *
 * Формат выходного файла (output.txt):
 *   - Одна строка на такт
 *   - Формат: "Tick N [OUT=1 ...]  state=cabin_state/motor_state"
 *
 * Логика работы лифта (elevator_mini.but):
 *   Cabin: Idle -> Moving -> AtFloor -> Idle
 *   Motor: Idle <-> Up | Down -> Stop -> Idle
 *   Разделяемые переменные: command, current_floor, target_floor
 */

#include "elevator_mini.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

/* ─── Константы ───────────────────────────────────────────────────────────── */
#define MAX_TICKS       512
#define IN_PORT_COUNT   40
#define OUT_PORT_COUNT  4
#define MAX_LINE        1024

/* ─── Состояние симулятора ─────────────────────────────────────────────────── */
/* inputs[tick][port] — входные порты по тактам */
static bool inputs[MAX_TICKS][IN_PORT_COUNT];
/* outputs[port] — выходные порты текущего такта */
static bool outputs[OUT_PORT_COUNT];
/* Номер текущего такта симуляции (1-based) */
static int current_tick = 0;
/* Общее число считанных тактов из файла */
static int total_ticks  = 0;
/* Файл вывода */
static FILE *out_fp = NULL;

/* ─── Таблица имён входных портов ──────────────────────────────────────────── */
typedef struct { const char *name; int index; } InPortEntry;

static const InPortEntry in_port_table[] = {
    /* Кнопки кабины */
    { "CabinButton_DC",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_DC  },
    { "CabinButton_DO",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_DO  },
    { "CabinButton_F1",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F1  },
    { "CabinButton_F2",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F2  },
    { "CabinButton_F3",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F3  },
    { "CabinButton_F4",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F4  },
    { "CabinButton_F5",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F5  },
    { "CabinButton_F6",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F6  },
    { "CabinButton_F7",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F7  },
    { "CabinButton_F8",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F8  },
    { "CabinButton_F9",            ELEVATOR_MINI_CABIN_CABIN_BUTTON_F9  },
    /* Кнопки вызова на этажах */
    { "FloorButton_F1",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F1  },
    { "FloorButton_F2",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F2  },
    { "FloorButton_F3",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F3  },
    { "FloorButton_F4",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F4  },
    { "FloorButton_F5",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F5  },
    { "FloorButton_F6",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F6  },
    { "FloorButton_F7",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F7  },
    { "FloorButton_F8",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F8  },
    { "FloorButton_F9",            ELEVATOR_MINI_CABIN_FLOOR_BUTTON_F9  },
    /* Датчики зоны этажа: нижняя граница */
    { "FloorSensor_F1_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F1_BOTTOM },
    { "FloorSensor_F2_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F2_BOTTOM },
    { "FloorSensor_F3_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F3_BOTTOM },
    { "FloorSensor_F4_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F4_BOTTOM },
    { "FloorSensor_F5_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F5_BOTTOM },
    { "FloorSensor_F6_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F6_BOTTOM },
    { "FloorSensor_F7_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F7_BOTTOM },
    { "FloorSensor_F8_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F8_BOTTOM },
    { "FloorSensor_F9_Bottom",     ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F9_BOTTOM },
    /* Датчики зоны этажа: верхняя граница */
    { "FloorSensor_F1_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F1_TOP    },
    { "FloorSensor_F2_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F2_TOP    },
    { "FloorSensor_F3_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F3_TOP    },
    { "FloorSensor_F4_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F4_TOP    },
    { "FloorSensor_F5_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F5_TOP    },
    { "FloorSensor_F6_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F6_TOP    },
    { "FloorSensor_F7_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F7_TOP    },
    { "FloorSensor_F8_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F8_TOP    },
    { "FloorSensor_F9_Top",        ELEVATOR_MINI_CABIN_FLOOR_SENSOR_F9_TOP    },
    /* Концевые датчики мотора */
    { "ElevatorMotor_SensorD",     ELEVATOR_MINI_MOTOR_ELEVATOR_MOTOR_SENSOR_D },
    { "ElevatorMotor_SensorU",     ELEVATOR_MINI_MOTOR_ELEVATOR_MOTOR_SENSOR_U },
};
static const int in_port_count = (int)(sizeof(in_port_table) / sizeof(in_port_table[0]));

/* ─── Таблица имён выходных портов ─────────────────────────────────────────── */
typedef struct { const char *name; int index; } OutPortEntry;

static const OutPortEntry out_port_table[] = {
    { "DoorOpen",              ELEVATOR_MINI_CABIN_DOOR_OPEN              },
    { "ElevatorMotor_Down",    ELEVATOR_MINI_MOTOR_ELEVATOR_MOTOR_DOWN    },
    { "ElevatorMotor_Stop",    ELEVATOR_MINI_MOTOR_ELEVATOR_MOTOR_STOP    },
    { "ElevatorMotor_Up",      ELEVATOR_MINI_MOTOR_ELEVATOR_MOTOR_UP      },
};
static const int out_port_count = (int)(sizeof(out_port_table) / sizeof(out_port_table[0]));

/* ─── Функции портов (callbacks) ───────────────────────────────────────────── */

static bool sim_read_bit(ElevatorMini_In_BitPort port, void *userdata) {
    (void)userdata;
    if (current_tick < 1 || current_tick > total_ticks) return false;
    return inputs[current_tick - 1][(int)port];
}

static void sim_write_bit(ElevatorMini_Out_BitPort port, bool val, void *userdata) {
    (void)userdata;
    outputs[(int)port] = val;
}

/* ─── Разбор сценария ──────────────────────────────────────────────────────── */

static int find_in_port(const char *name) {
    for (int i = 0; i < in_port_count; i++)
        if (strcmp(in_port_table[i].name, name) == 0)
            return in_port_table[i].index;
    return -1;
}

/* Разбирает одну строку сценария и заполняет inputs[tick_idx][]. */
static void parse_line(int tick_idx, char *line) {
    /* Обнуляем все входы для этого такта */
    memset(inputs[tick_idx], 0, sizeof(inputs[tick_idx]));

    char *token = strtok(line, " \t\r\n");
    while (token) {
        /* Ищем знак '=' */
        char *eq = strchr(token, '=');
        if (eq) {
            *eq = '\0';
            const char *port_name = token;
            int value = atoi(eq + 1);
            int idx = find_in_port(port_name);
            if (idx >= 0) {
                inputs[tick_idx][idx] = (value != 0);
            } else {
                fprintf(stderr, "Предупреждение: неизвестный порт '%s' в такте %d\n",
                        port_name, tick_idx + 1);
            }
        }
        token = strtok(NULL, " \t\r\n");
    }
}

/* Загружает сценарий из файла. Возвращает число тактов или -1 при ошибке. */
static int load_scenario(const char *filename) {
    FILE *fp = fopen(filename, "r");
    if (!fp) {
        fprintf(stderr, "Ошибка: не удалось открыть файл сценария '%s'\n", filename);
        return -1;
    }

    char line[MAX_LINE];
    int tick = 0;
    while (fgets(line, sizeof(line), fp) && tick < MAX_TICKS) {
        /* Убираем символ новой строки */
        line[strcspn(line, "\n")] = '\0';

        /* Пропускаем пустые строки и комментарии */
        char *p = line;
        while (*p == ' ' || *p == '\t') p++;
        if (*p == '\0' || *p == '#') continue;

        parse_line(tick, p);
        tick++;
    }
    fclose(fp);
    return tick;
}

/* ─── Вывод состояния ──────────────────────────────────────────────────────── */

static const char *cabin_state_name(int state) {
    switch (state) {
        case ELEVATOR_MINI_CABIN_INIT:     return "INIT";
        case ELEVATOR_MINI_CABIN_IDLE:     return "Idle";
        case ELEVATOR_MINI_CABIN_MOVING:   return "Moving";
        case ELEVATOR_MINI_CABIN_AT_FLOOR: return "AtFloor";
        case ELEVATOR_MINI_CABIN_END:      return "END";
        default:                           return "?";
    }
}

static const char *motor_state_name(int state) {
    switch (state) {
        case ELEVATOR_MINI_MOTOR_INIT:  return "INIT";
        case ELEVATOR_MINI_MOTOR_IDLE:  return "Idle";
        case ELEVATOR_MINI_MOTOR_UP:    return "Up";
        case ELEVATOR_MINI_MOTOR_DOWN:  return "Down";
        case ELEVATOR_MINI_MOTOR_STOP:  return "Stop";
        case ELEVATOR_MINI_MOTOR_END:   return "END";
        default:                        return "?";
    }
}

static void print_tick_result(FILE *fp, int tick, const ElevatorMini *fsm) {
    fprintf(fp, "Tick %3d |", tick);

    /* Входные порты, которые были активны в этом такте (только для такта >= 1) */
    fprintf(fp, " in=[");
    bool any_in = false;
    if (tick >= 1 && tick <= total_ticks) {
        for (int i = 0; i < in_port_count; i++) {
            if (inputs[tick - 1][in_port_table[i].index]) {
                fprintf(fp, "%s%s", any_in ? "," : "", in_port_table[i].name);
                any_in = true;
            }
        }
    }
    fprintf(fp, "]");

    /* Выходные порты, активированные в этом такте */
    fprintf(fp, " out=[");
    bool any_out = false;
    for (int i = 0; i < out_port_count; i++) {
        if (outputs[out_port_table[i].index]) {
            fprintf(fp, "%s%s", any_out ? "," : "", out_port_table[i].name);
            any_out = true;
        }
    }
    fprintf(fp, "]");

    /* Состояние FSM и разделяемые переменные */
    fprintf(fp, " cabin=%-8s motor=%-6s floor=%d target=%d cmd=%d\n",
            cabin_state_name((int)fsm->main.cabin0.state),
            motor_state_name((int)fsm->main.motor1.state),
            (int)fsm->current_floor,
            (int)fsm->target_floor,
            (int)fsm->command);
}

/* ─── Точка входа ──────────────────────────────────────────────────────────── */

int main(int argc, char *argv[]) {
    if (argc < 3) {
        fprintf(stderr,
                "Использование: %s <scenario.txt> <output.txt>\n"
                "  scenario.txt  — входные состояния портов по тактам\n"
                "  output.txt    — результаты симуляции\n",
                argv[0]);
        return 1;
    }

    /* Загружаем сценарий */
    total_ticks = load_scenario(argv[1]);
    if (total_ticks < 0) return 1;
    fprintf(stderr, "Загружено %d тактов из '%s'\n", total_ticks, argv[1]);

    /* Открываем файл вывода */
    out_fp = fopen(argv[2], "w");
    if (!out_fp) {
        fprintf(stderr, "Ошибка: не удалось открыть файл вывода '%s'\n", argv[2]);
        return 1;
    }

    /* Заголовок выходного файла */
    fprintf(out_fp, "# Симуляция 9-этажного лифта (elevator_mini)\n");
    fprintf(out_fp, "# Сценарий: %s  Тактов: %d\n", argv[1], total_ticks);
    fprintf(out_fp, "#\n");
    fprintf(out_fp, "# Колонки: такт | активные входы | активные выходы | состояние кабины | состояние мотора | этаж | цель | команда\n");
    fprintf(out_fp, "#\n");

    /* Инициализируем FSM */
    ElevatorMini fsm;
    memset(&fsm, 0, sizeof(fsm));
    fsm.read_bit  = sim_read_bit;
    fsm.write_bit = sim_write_bit;
    ElevatorMini_init(&fsm);

    /* Запись состояния после инициализации */
    memset(outputs, 0, sizeof(outputs));
    print_tick_result(out_fp, 0, &fsm);

    /* Основной цикл симуляции */
    for (int t = 1; t <= total_ticks; t++) {
        current_tick = t;
        /* Сбрасываем выходы перед каждым тактом */
        memset(outputs, 0, sizeof(outputs));

        ElevatorMini_tick(&fsm);

        print_tick_result(out_fp, t, &fsm);
    }

    fclose(out_fp);
    fprintf(stderr, "Результат записан в '%s'\n", argv[2]);
    return 0;
}

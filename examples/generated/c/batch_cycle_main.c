// Драйвер проверки порождённого C: технологический цикл на последовательной
// композиции `+` (examples/batch_cycle.takt).
//
// НЕ порождается taktc: харнесс — принадлежность ПРОВЕРКИ, а не продукта, как и
// тестбенчи цели sv.
//
// Проверяется не «программа не упала», а СУТЬ конструкции: три фазы обязаны
// пройти ОДНА ЗА ДРУГОЙ. Наблюдаемая — общая `stage` (номер активной фазы,
// 1 → 2 → 3): если бы `+` вела себя как `|`, номера пошли бы вперемешку. Порядок
// проверяется `assert`-ами, поэтому нарушение валит гейт цели `c`, а не просто
// печатает странный лог.
#include "batch_cycle.h"
#include <assert.h>
#include <stdio.h>

static int ready_seen = 0;

static void write_bit(BatchCycle_Out_BitPort port, bool val, void *userdata) {
    (void)port;
    (void)userdata;
    if (val) {
        ready_seen = 1;
    }
}

int main(void) {
    BatchCycle fsm;
    int i;
    int last_stage = 0;
    int phases = 0;

    fsm.write_bit = write_bit;
    fsm.userdata = NULL;
    BatchCycle_init(&fsm);

    for (i = 0; i < 100 && !BatchCycle_is_done(&fsm); i++) {
        BatchCycle_tick(&fsm);
        if (fsm.stage != last_stage) {
            /* Фаза сменилась. Номер обязан вырасти РОВНО на единицу: пропуск
               означал бы, что шаг цепочки не исполнился, а убывание — что фазы
               идут не по порядку. */
            assert(fsm.stage == last_stage + 1);
            last_stage = fsm.stage;
            phases++;
            printf("фаза %d\n", (int)fsm.stage);
        }
    }

    assert(phases == 3);   /* дозирование, перемешивание, слив */
    assert(ready_seen);    /* готовность поднята после последней фазы */
    assert(BatchCycle_is_done(&fsm));

    printf("Цикл: три фазы по порядку, завершено за %d шагов\n", i);
    return 0;
}

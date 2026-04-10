#ifndef LSP_DEMO_H__
#define LSP_DEMO_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct LspDemoRobot LspDemoRobot;
typedef struct LspDemo LspDemo;

// NOTICE: Определение констант для модели Robot (LspDemo:Robot)
/* Model Robot (LspDemo:Robot) */
struct LspDemoRobot {
    // NOTICE: Определение переменных модели
    int active;
    uint8_t speed;
    enum {
        LSP_DEMO_ROBOT_INIT,
        LSP_DEMO_ROBOT_MOVING,
        LSP_DEMO_ROBOT_IDLE,
        LSP_DEMO_ROBOT_END
    } state;
};

// NOTICE: Определение констант для модели lsp_demo (LspDemo)
/* Model lsp_demo (LspDemo) */
struct LspDemo {
    // NOTICE: Определение переменных модели
    uint8_t heading;
    enum {
        LSP_DEMO_INIT,
        LSP_DEMO_MAIN,
        LSP_DEMO_END
    } state;
    // NOTICE: Определение extend
    LspDemoRobot main;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void LspDemo_init(LspDemo *main);
void LspDemo_tick(LspDemo *main);
void LspDemo_reset(LspDemo *main);
bool LspDemo_is_done(const LspDemo *main);
#endif

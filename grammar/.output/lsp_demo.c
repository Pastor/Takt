#include "lsp_demo.h"
#include <math.h>
/// Перечисления модели lsp_demo (LspDemo)
#define ENUM_LSP_DEMO_EAST 2
#define ENUM_LSP_DEMO_HIGH 10
#define ENUM_LSP_DEMO_LOW 0
#define ENUM_LSP_DEMO_MEDIUM 5
#define ENUM_LSP_DEMO_NORTH 0
#define ENUM_LSP_DEMO_SOUTH 1
#define ENUM_LSP_DEMO_WEST 3
///Внешние функции
extern void log(uint8_t msg);

void LspDemo_init(LspDemo *main) {
    main->state = LSP_DEMO_INIT;
}

void LspDemo_tick(LspDemo *main) {
}

void LspDemo_reset(LspDemo *main) {
    LspDemo_init(main);
}

bool LspDemo_is_done(const LspDemo *main) {
    return main->state == LSP_DEMO_MAIN;
}


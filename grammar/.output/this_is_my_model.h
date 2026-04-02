#ifndef THIS_IS_MY_MODEL__
#define THIS_IS_MY_MODEL__
#include <stdint.h>
#include <stdbool.h>

struct ThisIsMyModel {
    struct {
        enum {
            THIS_IS_MY_MODEL_PING_INIT,
            THIS_IS_MY_MODEL_PING_START,
            THIS_IS_MY_MODEL_PING_END
        } state;
        bool toggle;
    } ping;
    struct {
        enum {
            THIS_IS_MY_MODEL_PONG_INIT,
            THIS_IS_MY_MODEL_PONG_STOP,
            THIS_IS_MY_MODEL_PONG_BEGIN
        } state;
    } pong;
    struct {
        enum {
            THIS_IS_MY_MODEL_TOGGLE_INIT,
            THIS_IS_MY_MODEL_TOGGLE_COMPLETE,
            THIS_IS_MY_MODEL_TOGGLE_ENTRY,
            THIS_IS_MY_MODEL_TOGGLE_END,
            THIS_IS_MY_MODEL_TOGGLE_PONG,
            THIS_IS_MY_MODEL_TOGGLE_PING
        } state;
    } toggle;
    enum {
        THIS_IS_MY_MODEL_INIT,
        THIS_IS_MY_MODEL_ENTRY
    } state;
    uint64_t it;
    void *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};
void ThisIsMyModel_init(struct ThisIsMyModel *main);
void ThisIsMyModel_tick(struct ThisIsMyModel *main);
void ThisIsMyModel_reset(struct ThisIsMyModel *main);
bool ThisIsMyModel_is_done(const struct ThisIsMyModel *main);
#endif

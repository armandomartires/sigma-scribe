/* Thin C shim over FBInk so Rust never mirrors FBInkConfig's layout.
 * SPDX-License-Identifier: AGPL-3.0-only (FBInk itself is GPL-3.0-or-later). */
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <sys/ioctl.h>
#include <linux/input.h>
#include "fbink.h"

static int         g_fd = -1;
static FBInkConfig g_cfg;

int sg_open(uint32_t* w, uint32_t* h)
{
	memset(&g_cfg, 0, sizeof g_cfg);
	g_fd = fbink_open();
	if (g_fd < 0)
		return -1;
	if (fbink_init(g_fd, &g_cfg) < 0)
		return -2;
	FBInkState st;
	memset(&st, 0, sizeof st);
	fbink_get_state(&g_cfg, &st);
	*w = st.screen_width;
	*h = st.screen_height;
	return 0;
}

/* Copies an 8-bit grey rectangle into the framebuffer without refreshing. */
int sg_blit(const unsigned char* data, int w, int h, int x, int y)
{
	g_cfg.no_refresh = true;
	int r = fbink_print_raw_data(g_fd, data, w, h, (size_t) w * (size_t) h, (short int) x, (short int) y, &g_cfg);
	g_cfg.no_refresh = false;
	return r;
}

/* mode 0 = fast (typing), 1 = partial GC16_FAST, 2 = full flashing GC16,
 * 3 = moving pointer (DU, v1.0.3), 4 = pointer cleanup (GL16, v1.0.3).
 * Never waits for completion: the input loop must not block (v1.0.3). */
int sg_refresh(uint32_t x, uint32_t y, uint32_t w, uint32_t h, int mode)
{
	g_cfg.is_flashing = (mode == 2);
	g_cfg.wfm_mode    = mode == 0 ? WFM_AUTO : (mode == 1 ? WFM_GC16_FAST : (mode == 3 ? WFM_DU : (mode == 4 ? WFM_GL16 : (mode == 5 ? WFM_A2 : WFM_GC16))));
	int r             = fbink_refresh(g_fd, y, x, w, h, &g_cfg);
	g_cfg.is_flashing = false;
	return r;
}

/* Maximum of an absolute axis (touch range), or -1. */
int sg_abs_max(int fd, int code)
{
	struct input_absinfo info;
	if (ioctl(fd, EVIOCGABS(code), &info) < 0)
		return -1;
	return info.maximum;
}

int sg_close(void)
{
	return fbink_close(g_fd);
}

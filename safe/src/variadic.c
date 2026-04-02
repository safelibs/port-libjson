#define _GNU_SOURCE

#include <errno.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <syslog.h>

#include "json_pointer.h"
#include "printbuf.h"

extern int mc_get_debug(void);
extern int __json_c_get_syslog_enabled(void);
extern void __json_c_set_last_err_text(const char *text);

static int format_string_alloc(char **out, const char *fmt, va_list ap)
{
    va_list ap_copy;
    int needed;

    *out = NULL;

    va_copy(ap_copy, ap);
    needed = vsnprintf(NULL, 0, fmt, ap_copy);
    va_end(ap_copy);
    if (needed < 0)
        return needed;

    *out = malloc((size_t)needed + 1);
    if (*out == NULL)
    {
        errno = ENOMEM;
        return -1;
    }

    va_copy(ap_copy, ap);
    if (vsnprintf(*out, (size_t)needed + 1, fmt, ap_copy) < 0)
    {
        va_end(ap_copy);
        free(*out);
        *out = NULL;
        return -1;
    }
    va_end(ap_copy);
    return needed;
}

void mc_debug(const char *msg, ...)
{
    va_list ap;

    if (!mc_get_debug())
        return;

    va_start(ap, msg);
    if (__json_c_get_syslog_enabled())
        vsyslog(LOG_DEBUG, msg, ap);
    else
        vprintf(msg, ap);
    va_end(ap);
}

void mc_error(const char *msg, ...)
{
    va_list ap;

    va_start(ap, msg);
    if (__json_c_get_syslog_enabled())
        vsyslog(LOG_ERR, msg, ap);
    else
        vfprintf(stderr, msg, ap);
    va_end(ap);
}

void mc_info(const char *msg, ...)
{
    va_list ap;

    va_start(ap, msg);
    if (__json_c_get_syslog_enabled())
        vsyslog(LOG_INFO, msg, ap);
    else
        vfprintf(stderr, msg, ap);
    va_end(ap);
}

int sprintbuf(struct printbuf *p, const char *msg, ...)
{
    va_list ap;
    va_list ap_copy;
    char buf[128];
    char *dyn = NULL;
    int size;

    va_start(ap, msg);
    va_copy(ap_copy, ap);
    size = vsnprintf(buf, sizeof(buf), msg, ap);
    va_end(ap);

    if (size < 0 || size > 127)
    {
        if (size < 0)
        {
            size = vsnprintf(NULL, 0, msg, ap_copy);
            if (size < 0)
            {
                va_end(ap_copy);
                return -1;
            }
        }

        dyn = malloc((size_t)size + 1);
        if (dyn == NULL)
        {
            va_end(ap_copy);
            return -1;
        }

        if (vsnprintf(dyn, (size_t)size + 1, msg, ap_copy) < 0)
        {
            va_end(ap_copy);
            free(dyn);
            return -1;
        }

        va_end(ap_copy);
        size = printbuf_memappend(p, dyn, size);
        free(dyn);
        return size;
    }

    va_end(ap_copy);
    return printbuf_memappend(p, buf, size);
}

void _json_c_set_last_err(const char *err_fmt, ...)
{
    va_list ap;
    char buf[256];

    va_start(ap, err_fmt);
    if (vsnprintf(buf, sizeof(buf), err_fmt, ap) < 0)
        buf[0] = '\0';
    va_end(ap);

    buf[sizeof(buf) - 1] = '\0';
    __json_c_set_last_err_text(buf);
}

int json_pointer_getf(struct json_object *obj, struct json_object **res, const char *path_fmt, ...)
{
    va_list ap;
    char *path = NULL;
    int rc;

    if (!obj || !path_fmt)
    {
        errno = EINVAL;
        return -1;
    }

    va_start(ap, path_fmt);
    rc = format_string_alloc(&path, path_fmt, ap);
    va_end(ap);
    if (rc < 0)
        return rc;

    rc = json_pointer_get(obj, path, res);
    free(path);
    return rc;
}

int json_pointer_setf(struct json_object **obj, struct json_object *value, const char *path_fmt, ...)
{
    va_list ap;
    char *path = NULL;
    int rc;

    if (!obj || !path_fmt)
    {
        errno = EINVAL;
        return -1;
    }

    va_start(ap, path_fmt);
    rc = format_string_alloc(&path, path_fmt, ap);
    va_end(ap);
    if (rc < 0)
        return rc;

    rc = json_pointer_set(obj, path, value);
    free(path);
    return rc;
}

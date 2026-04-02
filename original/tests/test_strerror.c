#ifdef NDEBUG
#undef NDEBUG
#endif
#include <errno.h>
#include <stdio.h>

static const char *errno_name(int errnum)
{
	switch (errnum)
	{
	case EINVAL: return "EINVAL";
	case ENOENT: return "ENOENT";
	case EBADF: return "EBADF";
	case ENOMEM: return "ENOMEM";
	default: break;
	}

	static char buf[32];
	(void)snprintf(buf, sizeof(buf), "%d", errnum);
	return buf;
}

static const char *format_errno(int errnum)
{
	static char buf[40];
	(void)snprintf(buf, sizeof(buf), "ERRNO=%s", errno_name(errnum));
	return buf;
}

int main(int argc, char **argv)
{
	(void)argc;
	(void)argv;
	puts(format_errno(10000));
	puts(format_errno(999));
	return 0;
}

#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>

#include "json_util.h"

int main(int argc, char **argv)
{
	struct json_object *jso;
	const char *last_err;

	(void)argc;
	(void)argv;

	jso = json_object_from_file("not_present.json");
	assert(jso == NULL);
	last_err = json_util_get_last_err();
	assert(last_err != NULL);
	fputs(last_err, stdout);

	jso = json_object_from_fd(-1);
	assert(jso == NULL);
	last_err = json_util_get_last_err();
	assert(last_err != NULL);
	fputs(last_err, stdout);

	return 0;
}

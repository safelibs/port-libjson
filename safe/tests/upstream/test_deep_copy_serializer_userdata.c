#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "json.h"

int main(void)
{
	struct json_object *src = json_object_new_double(1.5);
	struct json_object *dst = NULL;
	const char *last_err;

	assert(src != NULL);
	json_object_set_serializer(src, json_object_double_to_json_string, NULL,
	                           json_object_free_userdata);

	assert(-1 == json_object_deep_copy(src, &dst, NULL));
	assert(dst == NULL);

	last_err = json_util_get_last_err();
	assert(last_err != NULL);
	assert(strstr(last_err, "unable to copy unknown serializer data") != NULL);

	printf("deep_copy with NULL serializer userdata failed as expected.\n");

	json_object_put(src);
	return 0;
}

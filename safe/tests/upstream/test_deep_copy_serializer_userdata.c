#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#include "json.h"

static void test_unknown_serializer_with_null_userdata(void)
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
	assert(strncmp(last_err,
	               "json_object_copy_serializer_data: unable to copy unknown serializer data: ",
	               strlen("json_object_copy_serializer_data: unable to copy unknown serializer data: ")) == 0);
	{
		void *serializer = NULL;
		int consumed = 0;

		assert(sscanf(last_err,
		              "json_object_copy_serializer_data: unable to copy unknown serializer data: %p%n",
		              &serializer, &consumed) == 1);
		assert(serializer != NULL);
		assert(last_err[consumed] == '\n');
		assert(last_err[consumed + 1] == '\0');
	}

	json_object_put(src);
}

static void test_public_userdata_serializer_aborts(void)
{
	pid_t child = fork();
	int status = 0;

	assert(child >= 0);
	if (child == 0)
	{
		struct json_object *src = json_object_new_double(1.5);
		struct json_object *dst = NULL;

		assert(src != NULL);
		json_object_set_serializer(src, json_object_userdata_to_json_string, NULL,
		                           json_object_free_userdata);
		(void)json_object_deep_copy(src, &dst, NULL);
		_exit(0);
	}

	assert(waitpid(child, &status, 0) == child);
	assert(WIFSIGNALED(status));
	assert(WTERMSIG(status) == SIGABRT);
}

int main(void)
{
	test_unknown_serializer_with_null_userdata();
	test_public_userdata_serializer_aborts();
	printf("deep_copy serializer userdata edge cases matched upstream.\n");
	return 0;
}

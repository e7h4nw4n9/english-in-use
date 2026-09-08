<script setup lang="ts">
import { ref, watch } from 'vue'
import { message } from 'ant-design-vue'
import { getDictionaryAuthStatus, loginDictionary, sendDictionaryVerifyCode } from '@/lib/api'
import { getReadableCommandError } from '@/lib/error'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{
  (event: 'cancel'): void
  (event: 'success'): void
}>()

const phone = ref('')
const code = ref('')
const sending = ref(false)
const loggingIn = ref(false)

/** 加载上次成功登录的手机号，减少重复输入。 */
async function loadCachedPhone() {
  try {
    const status = await getDictionaryAuthStatus()
    phone.value = status.cachedPhone || ''
  } catch (error) {
    message.error(getReadableCommandError(error))
  }
}

/** 向当前手机号发送登录验证码。 */
async function sendCode() {
  if (!phone.value.trim() || sending.value) return
  sending.value = true
  try {
    await sendDictionaryVerifyCode(phone.value)
    message.success('验证码已发送')
  } catch (error) {
    message.error(getReadableCommandError(error))
  } finally {
    sending.value = false
  }
}

/** 登录词典账号并通知调用方继续原查询。 */
async function login() {
  if (!phone.value.trim() || !code.value.trim() || loggingIn.value) return
  loggingIn.value = true
  try {
    await loginDictionary(phone.value, code.value)
    code.value = ''
    message.success('词典登录成功')
    emit('success')
  } catch (error) {
    message.error(getReadableCommandError(error))
  } finally {
    loggingIn.value = false
  }
}

watch(
  () => props.open,
  (open) => {
    if (open) void loadCachedPhone()
  },
  { immediate: true },
)
</script>

<template>
  <a-modal
    :open="open"
    title="登录牛津词典"
    :confirm-loading="loggingIn"
    ok-text="登录"
    cancel-text="取消"
    centered
    @ok="login"
    @cancel="emit('cancel')"
  >
    <a-alert class="login-alert" type="info" show-icon message="登录后可继续查询牛津高阶词典" />
    <a-form layout="vertical" @finish="login">
      <a-form-item label="手机号" required>
        <a-input v-model:value="phone" autocomplete="tel" inputmode="tel" />
      </a-form-item>
      <a-form-item label="验证码" required>
        <a-input v-model:value="code" autocomplete="one-time-code" @press-enter="login">
          <template #addonAfter>
            <a-button type="link" size="small" :loading="sending" @click="sendCode">
              获取验证码
            </a-button>
          </template>
        </a-input>
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<style scoped>
.login-alert {
  margin-bottom: 18px;
}
</style>

<script setup lang="ts">
import type { KeyUsageAccountDetail } from '@/api/modules/key-usage'
import { shallowRef, watch } from 'vue'
import { getKeyUsageAccountDetail } from '@/api/modules/key-usage'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseSkeleton from '@/components/base/BaseSkeleton.vue'
import { useRequestState } from '@/composables/useRequestState'
import AccountExpandedPanels from '@/views/accounts/components/AccountExpandedPanels.vue'
import AccountQuotaDetails from '@/views/accounts/components/AccountQuotaPanel/Details.vue'
import AccountUsageDetails from '@/views/accounts/components/AccountUsageDetails.vue'

const props = defineProps<{ accountId: string }>()
const account = shallowRef<KeyUsageAccountDetail>()
const request = useRequestState()
const { loading, error } = request

// 每个展开行独立请求 卸载后取消并丢弃旧响应
async function load() {
  const id = request.start()
  account.value = undefined
  try {
    const result = await getKeyUsageAccountDetail(props.accountId, { signal: request.signal, silent: true })
    if (request.isCurrent(id))
      account.value = result
  }
  catch (cause) {
    request.fail(id, cause)
  }
  finally {
    request.finish(id)
  }
}

watch(() => props.accountId, load, { immediate: true })
</script>

<template>
  <div v-if="loading" class="grid gap-3 p-4 lg:grid-cols-3" aria-label="加载账号详情" aria-busy="true">
    <BaseSkeleton v-for="index in 3" :key="index" class="h-64 rounded-cp-lg" />
  </div>
  <div v-else-if="error" role="alert" class="flex items-center justify-center gap-3 p-6">
    <span class="text-cp-sm text-cp-error-text">{{ error }}</span>
    <BaseButton variant="secondary" @click="load">
      重试
    </BaseButton>
  </div>
  <AccountExpandedPanels v-else-if="account">
    <template #quota>
      <AccountQuotaDetails :account="account" />
    </template>
    <template #usage>
      <AccountUsageDetails :usage="account.usage" />
    </template>
  </AccountExpandedPanels>
</template>

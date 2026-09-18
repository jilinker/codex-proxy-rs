<script setup lang="ts">
import type { KeyUsageAccountDetail } from '@/api/modules/key-usage'
import BaseEmpty from '@/components/base/BaseEmpty.vue'
import BaseModal from '@/components/base/BaseModal/index.vue'
import BaseSkeleton from '@/components/base/BaseSkeleton.vue'
import AccountPlanBadge from '@/views/accounts/components/AccountPlanBadge.vue'

defineProps<{ account?: KeyUsageAccountDetail, loading: boolean, error: string }>()
const open = defineModel<boolean>({ required: true })
</script>

<template>
  <BaseModal v-model="open" title="账号详情" description="只读查看当前 Key 可用账号的额度与用量" size="lg">
    <div v-if="loading" class="grid gap-3">
      <BaseSkeleton v-for="index in 5" :key="index" class="h-14 rounded-cp" />
    </div>
    <BaseEmpty v-else-if="error || !account" :title="error ? '账号详情加载失败' : '暂无账号详情'" :description="error || undefined" />
    <div v-else class="grid gap-5">
      <section class="flex flex-wrap items-center gap-3 rounded-cp-lg bg-cp-fill-alter p-4">
        <div class="min-w-0 flex-1">
          <p class="m-0 truncate font-mono text-cp-lg font-heavy text-cp-text">
            {{ account.identity || '未提供账号标识' }}
          </p>
          <p class="mt-1 mb-0 text-cp-sm font-emphasis text-cp-text-secondary">
            {{ account.provider }} · {{ account.authenticationKind === 'api_key' ? 'API Key' : 'OAuth' }}
          </p>
        </div>
        <AccountPlanBadge :plan-type="account.planType" :plan-type-display="account.planTypeDisplay" :authentication-kind="account.authenticationKind" />
      </section>

      <section>
        <h3 class="mt-0 mb-3 text-cp-lg font-heavy text-cp-text">
          额度窗口
        </h3>
        <div v-if="account.quota.windows.length" class="grid gap-2 md:grid-cols-2">
          <article v-for="window in account.quota.windows" :key="window.key" class="rounded-cp-lg bg-cp-fill-alter p-4">
            <div class="flex items-start justify-between gap-3">
              <strong class="text-cp font-heavy text-cp-text">{{ window.labelDisplay }}</strong>
              <span class="font-mono text-cp-sm font-heavy tabular-nums" :class="window.limitReached ? 'text-cp-error-text' : 'text-cp-text'">{{ window.usedPercentDisplay }}</span>
            </div>
            <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-cp-fill-tertiary">
              <span class="block h-full rounded-full" :class="window.limitReached ? 'bg-cp-error' : 'bg-cp-primary'" :style="{ width: `${Math.min(Math.max(window.usedPercent ?? 0, 0), 100)}%` }" />
            </div>
            <div class="mt-2 flex justify-between gap-3 text-cp-xs font-emphasis text-cp-text-tertiary">
              <span>{{ window.windowLabelDisplay }}</span>
              <span>重置 {{ window.resetAtDisplay }}</span>
            </div>
          </article>
        </div>
        <BaseEmpty v-else title="暂无额度窗口" size="sm" />
      </section>

      <section>
        <h3 class="mt-0 mb-3 text-cp-lg font-heavy text-cp-text">
          用量概览
        </h3>
        <div class="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
          <div
            v-for="item in [
              ['请求数', account.usage.requestCountDisplay],
              ['总 Token', account.usage.totalTokensDisplay],
              ['成功率', account.usage.successRateDisplay],
              ['最近使用', account.usage.lastUsedAtDisplay],
            ]" :key="item[0]" class="rounded-cp-lg bg-cp-fill-alter p-3.5"
          >
            <span class="text-cp-xs font-bold text-cp-text-tertiary">{{ item[0] }}</span>
            <strong class="mt-1 block font-mono text-cp-lg font-heavy text-cp-text">{{ item[1] }}</strong>
          </div>
        </div>
      </section>

      <section v-if="account.usage.models.length">
        <h3 class="mt-0 mb-3 text-cp-lg font-heavy text-cp-text">
          模型用量
        </h3>
        <div class="overflow-hidden rounded-cp-lg bg-cp-fill-alter">
          <div v-for="model in account.usage.models" :key="model.model" class="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-4 border-b border-cp-border-subtle px-4 py-3 last:border-b-0">
            <code class="truncate font-mono text-cp-sm font-heavy text-cp-text">{{ model.model }}</code>
            <span class="text-cp-sm font-emphasis text-cp-text-secondary">{{ model.requestCountDisplay }} 次</span>
            <span class="font-mono text-cp-sm font-heavy text-cp-text">{{ model.totalTokensDisplay }}</span>
          </div>
        </div>
      </section>
    </div>
  </BaseModal>
</template>

<script setup lang="ts">
import type { AccountStatus } from '@/api/modules/accounts'
import type { KeyUsageAccount, KeyUsageAccountScopeState } from '@/api/modules/key-usage'
import type { BaseTablePagination as Pagination } from '@/components/base/BaseTable/pagination'
import { computed } from 'vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseTablePagination from '@/components/base/BaseTable/BaseTablePagination.vue'
import { defineTableColumns } from '@/components/base/BaseTable/columns'
import BaseTable from '@/components/base/BaseTable/index.vue'
import AccountPlanBadge from '@/views/accounts/components/AccountPlanBadge.vue'
import AccountStatusBadge from '@/views/accounts/components/AccountStatusBadge/index.vue'

const props = defineProps<{ rows: KeyUsageAccount[], pagination: Pagination, loading: boolean, error: string, scopeState: KeyUsageAccountScopeState }>()
defineEmits<{ pageChange: [page: number], pageSizeChange: [size: number], detail: [id: string] }>()
const columns = defineTableColumns<KeyUsageAccount>([
  { key: 'identity', label: '账号', kind: 'custom', size: '2xl' },
  { key: 'provider', label: '服务商', kind: 'status', size: 'lg' },
  { key: 'planTypeDisplay', label: '套餐', kind: 'custom', size: 'lg' },
  { key: 'status', label: '状态', kind: 'custom', size: 'lg' },
  { key: 'quota', label: '额度', kind: 'custom', size: '2xl' },
  { key: 'usage', label: '用量', kind: 'custom', size: '2xl' },
])
const emptyText = computed(() => {
  if (props.error)
    return '账号加载失败，请点击顶部刷新重试'
  if (props.scopeState === 'unbound')
    return '当前 Key 未绑定账号分组'
  if (props.scopeState === 'no_enabled_groups')
    return '当前 Key 没有已启用的账号分组'
  return '当前范围内暂无账号'
})
</script>

<template>
  <BaseCard title="账号用量" description="当前 Key 可用账号的额度与本地用量">
    <p v-if="error" role="alert" class="mt-0 mb-3 text-cp-sm text-cp-error-text">
      {{ error }}
    </p>
    <div class="flex h-120 min-h-0 overflow-hidden">
      <BaseTable
        class="min-w-0 flex-1"
        :columns="columns"
        :rows="rows"
        :loading="loading"
        :empty-text="emptyText"
        :row-action-label="row => `查看 ${row.identity || '账号'}详情`"
        scrollbar-always-visible
        @row-click="$emit('detail', $event.id)"
      >
        <template #identity="{ row }">
          <span class="block truncate font-mono text-cp-sm font-heavy text-cp-text">{{ row.identity || '—' }}</span>
        </template>
        <template #provider="{ row }">
          <span class="font-bold text-cp-text">{{ row.provider }}</span>
        </template>
        <template #planTypeDisplay="{ row }">
          <AccountPlanBadge size="sm" :plan-type="row.planType" :plan-type-display="row.planTypeDisplay" :authentication-kind="row.authenticationKind" />
        </template>
        <template #status="{ row }">
          <AccountStatusBadge :status="row.status as AccountStatus" />
        </template>
        <template #quota="{ row }">
          <div v-if="row.quota.windows[0]" class="grid gap-1">
            <div class="flex justify-between gap-2 text-cp-sm">
              <span class="truncate font-emphasis text-cp-text-secondary">{{ row.quota.windows[0].labelDisplay }}</span>
              <strong class="shrink-0 font-mono font-heavy text-cp-text">{{ row.quota.windows[0].usedPercentDisplay }}</strong>
            </div>
            <div class="h-1 rounded-full bg-cp-fill-tertiary">
              <span class="block h-full rounded-full bg-cp-primary" :style="{ width: `${Math.min(Math.max(row.quota.windows[0].usedPercent ?? 0, 0), 100)}%` }" />
            </div>
          </div>
          <span v-else class="text-cp-sm text-cp-text-tertiary">{{ row.quota.availability === 'unsupported' ? '上游未提供' : '暂无快照' }}</span>
        </template>
        <template #usage="{ row }">
          <div class="grid gap-0.5">
            <strong class="font-mono text-cp-sm font-heavy text-cp-text">{{ row.usage.totalTokensDisplay }} Tokens</strong>
            <span class="text-cp-xs font-emphasis text-cp-text-tertiary">{{ row.usage.requestCountDisplay }} 次 · 成功率 {{ row.usage.successRateDisplay }}</span>
          </div>
        </template>
      </BaseTable>
    </div>
    <BaseTablePagination :pagination="pagination" :loading="loading" @page-change="$emit('pageChange', $event)" @page-size-change="$emit('pageSizeChange', $event)" />
  </BaseCard>
</template>

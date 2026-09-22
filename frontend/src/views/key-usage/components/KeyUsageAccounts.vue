<script setup lang="ts">
import type { KeyUsageAccount, KeyUsageAccountScopeState } from '@/api/modules/key-usage'
import type { BaseTablePagination as Pagination } from '@/components/base/BaseTable/pagination'
import { ChevronDown } from '@lucide/vue'
import { computed, toRef, watch } from 'vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseTablePagination from '@/components/base/BaseTable/BaseTablePagination.vue'
import { defineTableColumns } from '@/components/base/BaseTable/columns'
import BaseTable from '@/components/base/BaseTable/index.vue'
import LastUsedAtCell from '@/components/LastUsedAtCell.vue'
import ProviderIconGroup from '@/components/ProviderIconGroup.vue'
import AccountPlanBadge from '@/views/accounts/components/AccountPlanBadge.vue'
import AccountQuotaSummaryCell from '@/views/accounts/components/AccountQuotaSummaryCell/index.vue'
import AccountStatusBadge from '@/views/accounts/components/AccountStatusBadge/index.vue'
import { useAccountsTable } from '@/views/accounts/composables/useAccountsTable'
import KeyUsageAccountDetail from './KeyUsageAccountDetail.vue'

const props = defineProps<{ rows: KeyUsageAccount[], pagination: Pagination, loading: boolean, error: string, scopeState: KeyUsageAccountScopeState }>()
defineEmits<{ pageChange: [page: number], pageSizeChange: [size: number] }>()
const { expandedAccountIds, expandedRowKeys, toggleExpanded } = useAccountsTable(toRef(props, 'rows'))
watch(() => props.rows, () => expandedAccountIds.value = new Set())
const columns = defineTableColumns<KeyUsageAccount>([
  { key: 'expander', kind: 'expander', hideable: false },
  { key: 'identity', label: '账号', kind: 'identity', size: '3xl' },
  { key: 'provider', label: '平台/类型', kind: 'meta', size: 'md', align: 'center' },
  { key: 'status', label: '状态', kind: 'status', align: 'left' },
  { key: 'planType', label: '套餐', kind: 'status' },
  { key: 'usage', label: '用量', kind: 'custom', size: '2xl' },
  { key: 'lastUsedAt', label: '最后使用', kind: 'datetime', emptyText: '' },
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
  <BaseCard title="账号用量">
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
        :expanded-row-keys="expandedRowKeys"
        scrollbar-always-visible
      >
        <template #expander="{ row }">
          <button
            type="button"
            class="inline-flex size-6 cursor-pointer items-center justify-center rounded-md border-0 bg-transparent text-cp-text-secondary hover:bg-cp-bg-text-hover hover:text-cp-text"
            :aria-label="`${expandedAccountIds.has(row.id) ? '收起' : '展开'} ${row.identity || '账号'}统计`"
            :aria-expanded="expandedAccountIds.has(row.id)"
            @click.stop="toggleExpanded(row.id)"
          >
            <ChevronDown class="size-3.5 transition-transform" :class="expandedAccountIds.has(row.id) ? '' : '-rotate-90'" />
          </button>
        </template>
        <template #identity="{ row }">
          <span class="block truncate font-mono text-cp-sm font-heavy text-cp-text">{{ row.identity || '—' }}</span>
        </template>
        <template #provider="{ row }">
          <ProviderIconGroup :provider="row.provider" :authentication-kind="row.authenticationKind" />
        </template>
        <template #planType="{ row }">
          <AccountPlanBadge size="sm" :plan-type="row.planType" :plan-type-display="row.planTypeDisplay" :authentication-kind="row.authenticationKind" />
        </template>
        <template #status="{ row }">
          <AccountStatusBadge :status="row.status" />
        </template>
        <template #usage="{ row }">
          <AccountQuotaSummaryCell :account="row" />
        </template>
        <template #lastUsedAt="{ row }">
          <LastUsedAtCell :value="row.usage.lastUsedAt" />
        </template>
        <template #expanded="{ row }">
          <KeyUsageAccountDetail :account-id="row.id" />
        </template>
      </BaseTable>
    </div>
    <BaseTablePagination :pagination="pagination" :loading="loading" @page-change="$emit('pageChange', $event)" @page-size-change="$emit('pageSizeChange', $event)" />
  </BaseCard>
</template>

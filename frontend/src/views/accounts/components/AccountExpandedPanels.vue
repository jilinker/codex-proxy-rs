<template>
  <div class="grid items-stretch gap-3 p-4 lg:grid-cols-[1.05fr_2.45fr] xl:min-h-77">
    <slot name="quota" />
    <slot name="usage" />
  </div>
</template>

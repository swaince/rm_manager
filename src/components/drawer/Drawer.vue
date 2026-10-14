<!-- eslint-disable vue/multi-word-component-names -- 公共组件统一命名为 Drawer。 -->
<script setup lang="ts">
import { computed } from 'vue'
import { XIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Drawer as DrawerRoot,
  DrawerClose,
  DrawerContent,
  DrawerFooter,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
} from '@/components/ui/drawer'

type DrawerSide = 'top' | 'right' | 'bottom' | 'left'
type DrawerSize = 'sm' | 'md' | 'lg'

const open = defineModel<boolean>('open', { default: false })

const props = withDefaults(
  defineProps<{
    title: string
    description?: string
    side?: DrawerSide
    size?: DrawerSize
    width?: number | string
  }>(),
  {
    side: 'right',
    size: 'md',
  },
)

const swipeDirection = computed(
  () =>
    (
      ({
        top: 'up',
        right: 'right',
        bottom: 'down',
        left: 'left',
      }) as const
    )[props.side],
)
const contentStyle = computed(() =>
  props.width === undefined
    ? undefined
    : {
        width: typeof props.width === 'number' ? `${props.width}px` : props.width,
        maxWidth: '100%',
      },
)

const verticalMaxWidthClasses: Record<DrawerSize, string> = {
  sm: 'data-[swipe-direction=left]:sm:max-w-sm data-[swipe-direction=right]:sm:max-w-sm',
  md: 'data-[swipe-direction=left]:sm:max-w-md data-[swipe-direction=right]:sm:max-w-md',
  lg: 'data-[swipe-direction=left]:sm:max-w-lg data-[swipe-direction=right]:sm:max-w-lg',
}

const horizontalHeightClasses: Record<DrawerSize, string> = {
  sm: 'h-80 data-[swipe-direction=up]:max-h-dvh data-[swipe-direction=down]:max-h-dvh',
  md: 'h-96 data-[swipe-direction=up]:max-h-dvh data-[swipe-direction=down]:max-h-dvh',
  lg: 'h-screen data-[swipe-direction=up]:max-h-dvh data-[swipe-direction=down]:max-h-dvh',
}

const contentSizeClass = computed(() => {
  if (props.side === 'left' || props.side === 'right') {
    return [
      'h-full data-[swipe-direction=left]:w-full data-[swipe-direction=right]:w-full',
      props.width === undefined ? verticalMaxWidthClasses[props.size] : '',
    ]
      .filter(Boolean)
      .join(' ')
  }

  return [
    horizontalHeightClasses[props.size],
    props.width === undefined
      ? ''
      : 'data-[swipe-direction=up]:mx-auto data-[swipe-direction=down]:mx-auto',
  ]
    .filter(Boolean)
    .join(' ')
})
</script>

<template>
  <DrawerRoot v-model:open="open" :swipe-direction="swipeDirection">
    <DrawerTrigger v-if="$slots.trigger" as-child>
      <slot name="trigger" />
    </DrawerTrigger>

    <DrawerContent
      class="min-h-0 max-h-dvh max-w-full w-full overflow-hidden bg-card text-card-foreground"
      :class="[
        contentSizeClass,
        (props.side === 'top' || props.side === 'bottom') &&
          'data-[swipe-direction=up]:mb-0 data-[swipe-direction=down]:mt-0',
      ]"
      :style="contentStyle"
    >
      <DrawerHeader
        class="form-header shrink-0 border-b border-border px-5 py-4 group-data-[swipe-direction=up]/drawer-content:text-left group-data-[swipe-direction=down]/drawer-content:text-left"
      >
        <div class="flex min-w-0 items-start justify-between gap-4">
          <div class="min-w-0 flex-1">
            <DrawerTitle v-if="$slots.header" class="sr-only">{{ props.title }}</DrawerTitle>
            <DrawerDescription v-if="$slots.header && props.description" class="sr-only">
              {{ props.description }}
            </DrawerDescription>
            <slot name="header">
              <DrawerTitle class="text-base font-semibold">{{ props.title }}</DrawerTitle>
              <DrawerDescription v-if="props.description" class="mt-1">
                {{ props.description }}
              </DrawerDescription>
            </slot>
          </div>
          <DrawerClose as-child>
            <Button
              type="button"
              aria-label="关闭抽屉"
              class="shrink-0"
              size="icon-sm"
              variant="ghost"
            >
              <XIcon aria-hidden="true" />
              <span class="sr-only">关闭</span>
            </Button>
          </DrawerClose>
        </div>
      </DrawerHeader>

      <ScrollArea
        data-drawer-body
        class="grid min-h-0 flex-1 [&>[data-slot=scroll-area-viewport]]:min-h-0 [&>[data-slot=scroll-area-viewport]]:h-auto"
      >
        <div class="px-5 py-4">
          <slot />
        </div>
      </ScrollArea>

      <DrawerFooter
        v-if="$slots.footer"
        class="shrink-0 flex-row flex-wrap items-center justify-end border-t border-border bg-form-footer px-5 py-3"
      >
        <slot name="footer" />
      </DrawerFooter>
    </DrawerContent>
  </DrawerRoot>
</template>

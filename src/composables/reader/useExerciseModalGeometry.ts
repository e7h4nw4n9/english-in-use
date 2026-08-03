import { computed, onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue'

/**
 * 管理练习弹窗的尺寸、位置、拖动、缩放和窗口监听。
 * @param exerciseVisible - 练习弹窗是否可见。
 */
export function useExerciseModalGeometry(exerciseVisible: Ref<boolean>) {
  const TOOLBAR_HEIGHT = 40
  const INITIAL_ASPECT_RATIO = 1.5
  const FIXED_MODAL_WIDTH = 350
  const FIXED_MODAL_HEIGHT = 650 // FIXED_MODAL_WIDTH * INITIAL_ASPECT_RATIO

  const viewportWidth = ref(typeof window !== 'undefined' ? window.innerWidth : 1280)
  const viewportHeight = ref(typeof window !== 'undefined' ? window.innerHeight : 800)
  const isMaximized = ref(false)
  const modalPosition = ref({ x: 0, y: 0 })
  const lastNormalPosition = ref<{ x: number; y: number } | null>(null)
  const manualSize = ref<{ width: number; height: number } | null>(null)

  const dragState = ref({
    dragging: false,
    startX: 0,
    startY: 0,
    originX: 0,
    originY: 0,
    pointerId: null as number | null,
  })
  const resizeState = ref({
    resizing: false,
    startX: 0,
    startY: 0,
    originWidth: 0,
    originHeight: 0,
    pointerId: null as number | null,
  })

  const isPhone = computed(() => viewportWidth.value < 768)
  const isTablet = computed(() => viewportWidth.value >= 768 && viewportWidth.value < 1100)
  const modalMarginRatio = computed(() => (isPhone.value ? 0.035 : isTablet.value ? 0.04 : 0.05))
  const modalMargin = computed(() =>
    Math.round(Math.min(viewportWidth.value, viewportHeight.value) * modalMarginRatio.value),
  )
  const availableWidth = computed(() => Math.max(0, viewportWidth.value - modalMargin.value * 2))
  const availableHeight = computed(() => Math.max(0, viewportHeight.value - modalMargin.value * 2))
  const minModalWidth = computed(() => availableWidth.value * (isPhone.value ? 0.88 : 0.15))
  const minModalHeight = computed(() => availableHeight.value * (isPhone.value ? 0.56 : 0.32))

  function fitSizeByAspect(maxWidth: number, maxHeight: number, heightPerWidth: number) {
    if (maxWidth <= 0 || maxHeight <= 0 || heightPerWidth <= 0) {
      return { width: 0, height: 0 }
    }

    let width = maxWidth
    let height = width * heightPerWidth
    if (height > maxHeight) {
      height = maxHeight
      width = height / heightPerWidth
    }
    return { width, height }
  }

  const defaultModalSize = computed(() => {
    if (isPhone.value) {
      // 手机：约 80% 窗口大小 (基于 availableWidth 计算)
      const targetWidth = availableWidth.value * 0.85
      const targetHeight = availableHeight.value * 0.85
      return fitSizeByAspect(targetWidth, targetHeight, INITIAL_ASPECT_RATIO)
    }

    // 平板/电脑：初始宽度不超过可用区域的三分之一
    const targetWidth = Math.min(FIXED_MODAL_WIDTH, availableWidth.value / 3)
    const targetHeight = Math.min(FIXED_MODAL_HEIGHT, availableHeight.value)
    return fitSizeByAspect(targetWidth, targetHeight, INITIAL_ASPECT_RATIO)
  })

  const defaultModalWidth = computed(() => defaultModalSize.value.width)
  const defaultModalHeight = computed(() => defaultModalSize.value.height)
  const canResize = computed(() => !isPhone.value && !isMaximized.value)

  const modalWidth = computed(() => {
    if (isMaximized.value) {
      return availableWidth.value
    }
    if (manualSize.value) {
      return Math.min(Math.max(manualSize.value.width, minModalWidth.value), availableWidth.value)
    }
    return Math.min(Math.max(defaultModalWidth.value, 0), availableWidth.value)
  })

  const modalHeight = computed(() => {
    if (isMaximized.value) {
      return availableHeight.value
    }
    if (manualSize.value) {
      return Math.min(
        Math.max(manualSize.value.height, minModalHeight.value),
        availableHeight.value,
      )
    }
    return Math.min(Math.max(defaultModalHeight.value, 0), availableHeight.value)
  })

  const modalBodyHeight = computed(() => `${Math.max(0, modalHeight.value - TOOLBAR_HEIGHT)}px`)

  const modalStyle = computed(() => ({
    top: `${modalPosition.value.y}px`,
    left: `${modalPosition.value.x}px`,
    margin: '0',
    paddingBottom: '0',
    position: 'fixed' as const,
  }))

  function clampPosition(x: number, y: number) {
    const margin = modalMargin.value
    const maxX = Math.max(margin, viewportWidth.value - modalWidth.value - margin)
    const maxY = Math.max(margin, viewportHeight.value - modalHeight.value - margin)
    const clampedX = Math.min(Math.max(margin, x), maxX)
    const clampedY = Math.min(Math.max(margin, y), maxY)
    return { x: clampedX, y: clampedY }
  }

  function centerModal() {
    const centeredX = (viewportWidth.value - modalWidth.value) / 2
    const centeredY = (viewportHeight.value - modalHeight.value) / 2
    modalPosition.value = clampPosition(centeredX, centeredY)
  }

  function stopDragging(event?: PointerEvent) {
    if (!dragState.value.dragging) return
    if (
      event &&
      dragState.value.pointerId !== null &&
      event.pointerId !== dragState.value.pointerId
    ) {
      return
    }

    dragState.value.dragging = false
    dragState.value.pointerId = null
    window.removeEventListener('pointermove', onDragPointerMove)
    window.removeEventListener('pointerup', stopDragging)
    window.removeEventListener('pointercancel', stopDragging)
  }

  function stopResizing(event?: PointerEvent) {
    if (!resizeState.value.resizing) return
    if (
      event &&
      resizeState.value.pointerId !== null &&
      event.pointerId !== resizeState.value.pointerId
    ) {
      return
    }
    resizeState.value.resizing = false
    resizeState.value.pointerId = null
    window.removeEventListener('pointermove', onResizePointerMove)
    window.removeEventListener('pointerup', stopResizing)
    window.removeEventListener('pointercancel', stopResizing)
  }

  function onResizePointerMove(event: PointerEvent) {
    if (!resizeState.value.resizing) return
    if (resizeState.value.pointerId !== null && event.pointerId !== resizeState.value.pointerId) {
      return
    }

    event.preventDefault()
    const deltaX = event.clientX - resizeState.value.startX
    const deltaY = event.clientY - resizeState.value.startY
    const nextWidth = resizeState.value.originWidth + deltaX
    const nextHeight = resizeState.value.originHeight + deltaY
    const width = Math.min(Math.max(nextWidth, minModalWidth.value), availableWidth.value)
    const height = Math.min(Math.max(nextHeight, minModalHeight.value), availableHeight.value)
    manualSize.value = { width, height }
    modalPosition.value = clampPosition(modalPosition.value.x, modalPosition.value.y)
  }

  function startResizing(event: PointerEvent) {
    if (!canResize.value || dragState.value.dragging) return
    if (event.pointerType === 'mouse' && event.button !== 0) return

    event.preventDefault()
    resizeState.value = {
      resizing: true,
      startX: event.clientX,
      startY: event.clientY,
      originWidth: modalWidth.value,
      originHeight: modalHeight.value,
      pointerId: event.pointerId,
    }

    window.addEventListener('pointermove', onResizePointerMove, { passive: false })
    window.addEventListener('pointerup', stopResizing)
    window.addEventListener('pointercancel', stopResizing)
  }

  function onDragPointerMove(event: PointerEvent) {
    if (!dragState.value.dragging || isMaximized.value) return
    if (dragState.value.pointerId !== null && event.pointerId !== dragState.value.pointerId) {
      return
    }

    event.preventDefault()
    const deltaX = event.clientX - dragState.value.startX
    const deltaY = event.clientY - dragState.value.startY
    modalPosition.value = clampPosition(
      dragState.value.originX + deltaX,
      dragState.value.originY + deltaY,
    )
  }

  function startDragging(event: PointerEvent) {
    if (isMaximized.value || resizeState.value.resizing) return
    if (event.pointerType === 'mouse' && event.button !== 0) return

    event.preventDefault()
    dragState.value = {
      dragging: true,
      startX: event.clientX,
      startY: event.clientY,
      originX: modalPosition.value.x,
      originY: modalPosition.value.y,
      pointerId: event.pointerId,
    }
    window.addEventListener('pointermove', onDragPointerMove, { passive: false })
    window.addEventListener('pointerup', stopDragging)
    window.addEventListener('pointercancel', stopDragging)
  }

  function closeModal() {
    exerciseVisible.value = false
  }

  function toggleMaximize() {
    if (!isMaximized.value) {
      lastNormalPosition.value = { ...modalPosition.value }
      stopResizing()
      isMaximized.value = true
      modalPosition.value = { x: modalMargin.value, y: modalMargin.value }
      return
    }

    isMaximized.value = false
    if (lastNormalPosition.value) {
      modalPosition.value = clampPosition(lastNormalPosition.value.x, lastNormalPosition.value.y)
    } else {
      centerModal()
    }
  }

  function onWindowResize() {
    viewportWidth.value = window.innerWidth
    viewportHeight.value = window.innerHeight

    if (manualSize.value) {
      manualSize.value = {
        width: Math.min(
          Math.max(manualSize.value.width, minModalWidth.value),
          availableWidth.value,
        ),
        height: Math.min(
          Math.max(manualSize.value.height, minModalHeight.value),
          availableHeight.value,
        ),
      }
    }

    if (isMaximized.value) {
      modalPosition.value = { x: modalMargin.value, y: modalMargin.value }
      return
    }
    modalPosition.value = clampPosition(modalPosition.value.x, modalPosition.value.y)
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && exerciseVisible.value) {
      closeModal()
    }
  }

  watch(exerciseVisible, (visible) => {
    if (visible) {
      isMaximized.value = false
      stopDragging()
      stopResizing()
      manualSize.value = null
      centerModal()
      return
    }
    stopDragging()
    stopResizing()
  })

  onMounted(() => {
    window.addEventListener('resize', onWindowResize)
    window.addEventListener('keydown', onWindowKeydown)
  })

  onBeforeUnmount(() => {
    stopDragging()
    stopResizing()
    window.removeEventListener('resize', onWindowResize)
    window.removeEventListener('keydown', onWindowKeydown)
  })

  return {
    isPhone,
    isMaximized,
    modalWidth,
    modalBodyHeight,
    modalStyle,
    canResize,
    startDragging,
    startResizing,
    closeModal,
    toggleMaximize,
  }
}

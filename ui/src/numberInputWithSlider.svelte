<script lang="ts">
    import NumberInput from "./numberInput.svelte";
    import Slider from "./slider.svelte";

    let {
        label,
        description,
        rightSide,
        value = $bindable(),
        units,
        step,
        min,
        max,
        labelWidth,
        labelBold,
        inputWidth,
        digits,
        paddingTop,
        onChange,
        sliderLogarithmic,
        sliderWidth,
        sliderTicks,
    }: {
        label: string;
        description: string;
        rightSide: boolean;
        value: number;
        units: string | null;
        step: number | "any";
        min: number | null;
        max: number | null;
        labelWidth: number;
        labelBold: boolean;
        inputWidth: number;
        digits: number;
        paddingTop: number;
        onChange?: (number) => void;
        sliderLogarithmic: boolean;
        sliderWidth: number;
        sliderTicks: number[];
    } = $props();
</script>

<div>
    <NumberInput
        {label}
        {description}
        {rightSide}
        bind:value
        {units}
        {step}
        {min}
        {max}
        {labelWidth}
        {labelBold}
        {inputWidth}
        {digits}
        {paddingTop}
        {onChange}
    ></NumberInput>
    <div class="spacer"></div>
    <Slider
        bind:value
        {step}
        {min}
        {max}
        {onChange}
        logarithmic={sliderLogarithmic}
        ticks={sliderTicks}
        width={sliderWidth}
    ></Slider>
</div>

<style>
    .spacer {
        height: 10px;
    }
</style>

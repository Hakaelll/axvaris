import { memo, useEffect, useRef, useState } from "react";
import * as THREE from "three";
import type { Connection } from "../../types";
import { useI18n } from "../../i18n";
import { groupVisualRoutes, routeLevel } from "../../lib/routes";
import { spherePoint, worldBorders } from "../../lib/world";

interface GlobeSceneProps {
  connections: Connection[];
  home: [number, number];
  selectedId: string | null;
  reduceMotion: boolean;
  smartFocus?: boolean;
  onSelect: (id: string) => void;
  onFps?: (fps: number) => void;
}

interface SceneState {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: GlobeControls;
  traffic: THREE.Group;
  animation: number;
  markerIds: Map<THREE.Object3D, string>;
  resizeObserver: ResizeObserver;
  pointer: THREE.Vector2;
  lastFrame: number;
  frameCount: number;
  fpsStarted: number;
  movingDots: Array<{
    mesh: THREE.Mesh;
    curve: THREE.QuadraticBezierCurve3;
    phase: number;
    speed: number;
  }>;
  reduceMotion: boolean;
  trafficSignature: string;
}

const SPAIN_LATITUDE = 40.4168;
const SPAIN_LONGITUDE = -3.7038;
const DEFAULT_CAMERA_DISTANCE = 3.05;

interface GlobeControls {
  target: THREE.Vector3;
  minDistance: number;
  maxDistance: number;
  update: () => void;
  dispose: () => void;
}

function createGlobeControls(
  camera: THREE.PerspectiveCamera,
  element: HTMLCanvasElement,
): GlobeControls {
  const spherical = new THREE.Spherical().setFromVector3(camera.position);
  const target = new THREE.Vector3();
  let drag: { pointerId: number; x: number; y: number; theta: number; phi: number } | null = null;
  const controls: GlobeControls = {
    target,
    minDistance: 1.85,
    maxDistance: 5.2,
    update: () => {
      spherical.radius = THREE.MathUtils.clamp(
        camera.position.distanceTo(target),
        controls.minDistance,
        controls.maxDistance,
      );
      camera.position.setFromSpherical(spherical).add(target);
      camera.lookAt(target);
    },
    dispose: () => {
      element.removeEventListener("pointerdown", pointerDown);
      element.removeEventListener("pointermove", pointerMove);
      element.removeEventListener("pointerup", pointerUp);
      element.removeEventListener("pointercancel", pointerUp);
      element.removeEventListener("wheel", wheel);
    },
  };
  const pointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return;
    element.setPointerCapture(event.pointerId);
    drag = {
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      theta: spherical.theta,
      phi: spherical.phi,
    };
  };
  const pointerMove = (event: PointerEvent) => {
    if (!drag || drag.pointerId !== event.pointerId) return;
    spherical.theta = drag.theta - (event.clientX - drag.x) * 0.006;
    spherical.phi = THREE.MathUtils.clamp(
      drag.phi - (event.clientY - drag.y) * 0.006,
      0.08,
      Math.PI - 0.08,
    );
    controls.update();
  };
  const pointerUp = (event: PointerEvent) => {
    if (drag?.pointerId === event.pointerId) drag = null;
  };
  const wheel = (event: WheelEvent) => {
    event.preventDefault();
    spherical.radius = THREE.MathUtils.clamp(
      spherical.radius * (event.deltaY > 0 ? 1.12 : 0.89),
      controls.minDistance,
      controls.maxDistance,
    );
    camera.position.setFromSpherical(spherical).add(target);
    camera.lookAt(target);
  };
  element.addEventListener("pointerdown", pointerDown);
  element.addEventListener("pointermove", pointerMove);
  element.addEventListener("pointerup", pointerUp);
  element.addEventListener("pointercancel", pointerUp);
  element.addEventListener("wheel", wheel, { passive: false });
  controls.update();
  return controls;
}

function centerCameraOnSpain(camera: THREE.PerspectiveCamera) {
  camera.position.set(
    ...spherePoint(SPAIN_LONGITUDE, SPAIN_LATITUDE, DEFAULT_CAMERA_DISTANCE),
  );
}

function makeBorderLines(): THREE.LineSegments {
  const vertices: number[] = [];
  for (const line of worldBorders()) {
    for (let index = 1; index < line.length; index += 1) {
      const previous = line[index - 1];
      const current = line[index];
      if (!previous || !current) continue;
      vertices.push(...spherePoint(previous[0], previous[1], 1.006));
      vertices.push(...spherePoint(current[0], current[1], 1.006));
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    "position",
    new THREE.Float32BufferAttribute(vertices, 3),
  );
  return new THREE.LineSegments(
    geometry,
    new THREE.LineBasicMaterial({
      color: 0x3a3d40,
      transparent: true,
      opacity: 0.72,
    }),
  );
}

function disposeGroup(group: THREE.Group) {
  for (const child of [...group.children]) {
    group.remove(child);
    child.traverse((object) => {
      const candidate = object as THREE.Mesh | THREE.Line;
      candidate.geometry?.dispose();
      const material = candidate.material;
      if (Array.isArray(material)) material.forEach((item) => item.dispose());
      else material?.dispose();
    });
  }
}

export const GlobeScene = memo(function GlobeScene({
  connections,
  home,
  selectedId,
  reduceMotion,
  smartFocus = true,
  onSelect,
  onFps,
}: GlobeSceneProps) {
  const t = useI18n();
  const hostRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<SceneState | null>(null);
  const onSelectRef = useRef(onSelect);
  const onFpsRef = useRef(onFps);
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    text: string;
  } | null>(null);

  useEffect(() => {
    onSelectRef.current = onSelect;
    onFpsRef.current = onFps;
  }, [onFps, onSelect]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(36, 1, 0.1, 100);
    centerCameraOnSpain(camera);
    const renderer = new THREE.WebGLRenderer({
      alpha: true,
      antialias: true,
      powerPreference: "high-performance",
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.append(renderer.domElement);

    const globe = new THREE.Mesh(
      new THREE.SphereGeometry(1, 64, 40),
      new THREE.MeshStandardMaterial({
        color: 0x141618,
        roughness: 0.88,
        metalness: 0.08,
      }),
    );
    scene.add(globe);
    scene.add(makeBorderLines());
    scene.add(new THREE.HemisphereLight(0x8e969f, 0x060708, 1.8));
    const side = new THREE.DirectionalLight(0xe4f222, 0.42);
    side.position.set(-2.4, 1.8, 2.2);
    scene.add(side);
    const rim = new THREE.Mesh(
      new THREE.SphereGeometry(1.035, 64, 40),
      new THREE.MeshBasicMaterial({
        color: 0x687078,
        transparent: true,
        opacity: 0.05,
        side: THREE.BackSide,
      }),
    );
    scene.add(rim);

    const traffic = new THREE.Group();
    scene.add(traffic);
    const controls = createGlobeControls(camera, renderer.domElement);
    const markerIds = new Map<THREE.Object3D, string>();
    const pointer = new THREE.Vector2();
    const projectedMarker = new THREE.Vector3();

    const markerAtPointer = () => {
      let closest: THREE.Object3D | null = null;
      let closestDistance = 0.0045;
      for (const marker of markerIds.keys()) {
        const worldPosition = marker.userData.worldPosition as
          | THREE.Vector3
          | undefined;
        if (!worldPosition) continue;
        projectedMarker.copy(worldPosition).project(camera);
        const x = projectedMarker.x - pointer.x;
        const y = projectedMarker.y - pointer.y;
        const distance = x * x + y * y;
        if (projectedMarker.z <= 1 && distance < closestDistance) {
          closest = marker;
          closestDistance = distance;
        }
      }
      return closest;
    };

    const resize = () => {
      const { width, height } = host.getBoundingClientRect();
      if (width <= 0 || height <= 0) return;
      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(host);
    resize();

    const handlePointer = (event: PointerEvent) => {
      const bounds = renderer.domElement.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      const hit = markerAtPointer();
      const id = hit ? markerIds.get(hit) : undefined;
      if (id) onSelectRef.current(id);
    };
    const handlePointerMove = (event: PointerEvent) => {
      const bounds = renderer.domElement.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      const hit = markerAtPointer();
      const text = hit?.userData.tooltip as string | undefined;
      renderer.domElement.style.cursor = text ? "pointer" : "grab";
      setTooltip(
        text
          ? {
              x: event.clientX - bounds.left + 14,
              y: event.clientY - bounds.top + 14,
              text,
            }
          : null,
      );
    };
    const handlePointerLeave = () => setTooltip(null);
    renderer.domElement.addEventListener("pointerup", handlePointer);
    renderer.domElement.addEventListener("pointermove", handlePointerMove);
    renderer.domElement.addEventListener("pointerleave", handlePointerLeave);

    const state: SceneState = {
      renderer,
      scene,
      camera,
      controls,
      traffic,
      markerIds,
      resizeObserver,
      pointer,
      animation: 0,
      lastFrame: performance.now(),
      frameCount: 0,
      fpsStarted: performance.now(),
      movingDots: [],
      reduceMotion,
      trafficSignature: "",
    };
    sceneRef.current = state;
    const animate = (now: number) => {
      state.frameCount += 1;
      if (now - state.fpsStarted >= 1_000) {
        onFpsRef.current?.(
          (state.frameCount * 1_000) / (now - state.fpsStarted),
        );
        state.frameCount = 0;
        state.fpsStarted = now;
      }
      state.lastFrame = now;
      controls.update();
      if (!state.reduceMotion) {
        const seconds = now / 1_000;
        for (const dot of state.movingDots) {
          const progress = (seconds * dot.speed + dot.phase) % 1;
          dot.mesh.position.copy(dot.curve.getPoint(progress));
        }
      }
      renderer.render(scene, camera);
      state.animation =
        document.visibilityState === "visible"
          ? requestAnimationFrame(animate)
          : 0;
    };
    const handleVisibility = () => {
      if (document.visibilityState === "visible" && state.animation === 0) {
        state.lastFrame = performance.now();
        state.animation = requestAnimationFrame(animate);
      }
    };
    document.addEventListener("visibilitychange", handleVisibility);
    if (document.visibilityState === "visible") {
      state.animation = requestAnimationFrame(animate);
    }

    return () => {
      cancelAnimationFrame(state.animation);
      document.removeEventListener("visibilitychange", handleVisibility);
      renderer.domElement.removeEventListener("pointerup", handlePointer);
      renderer.domElement.removeEventListener("pointermove", handlePointerMove);
      renderer.domElement.removeEventListener(
        "pointerleave",
        handlePointerLeave,
      );
      resizeObserver.disconnect();
      controls.dispose();
      disposeGroup(traffic);
      scene.traverse((object) => {
        const candidate = object as THREE.Mesh | THREE.Line;
        candidate.geometry?.dispose();
        const material = candidate.material;
        if (Array.isArray(material)) material.forEach((item) => item.dispose());
        else material?.dispose();
      });
      renderer.dispose();
      renderer.domElement.remove();
      sceneRef.current = null;
    };
  }, []);

  useEffect(() => {
    const state = sceneRef.current;
    if (!state) return;
    state.reduceMotion = reduceMotion;
    state.renderer.setPixelRatio(
      Math.min(window.devicePixelRatio, reduceMotion ? 1.25 : 2),
    );
    const [homeX, homeY, homeZ] = spherePoint(home[1], home[0], 1.012);
    const homePoint = new THREE.Vector3(homeX, homeY, homeZ);
    const located = connections.filter((connection) => connection.geo !== null);
    const routes = groupVisualRoutes(
      located,
      routeLevel(state.camera.position.length(), located.length),
    ).slice(0, 120);
    const signature = `${home.join(":")}|${selectedId ?? ""}|${routes
      .map((route) => `${route.key}:${route.count}:${route.active}`)
      .join("|")}`;
    if (signature === state.trafficSignature) return;
    state.trafficSignature = signature;
    disposeGroup(state.traffic);
    state.markerIds.clear();
    state.movingDots = [];
    for (const route of routes) {
      const connection = route.representative;
      const selected =
        selectedId !== null && route.memberIds.includes(selectedId);
      const destination = new THREE.Vector3(
        ...spherePoint(route.longitude, route.latitude, 1.012),
      );
      const midpoint = homePoint
        .clone()
        .add(destination)
        .normalize()
        .multiplyScalar(
          1.13 + Math.min(0.44, homePoint.distanceTo(destination) * 0.16),
        );
      const curve = new THREE.QuadraticBezierCurve3(
        homePoint,
        midpoint,
        destination,
      );
      const geometry = new THREE.BufferGeometry().setFromPoints(
        curve.getPoints(selected ? 44 : 28),
      );
      const material = new THREE.LineBasicMaterial({
        color: selected ? 0xe4f222 : route.active ? 0x8f989c : 0x555b5e,
        transparent: true,
        opacity: selected ? 1 : route.active ? 0.55 : 0.2,
        depthTest: true,
      });
      state.traffic.add(new THREE.Line(geometry, material));

      const radius = selected
        ? 0.026
        : Math.min(0.031, 0.015 + Math.log2(route.count + 1) * 0.003);
      const markerGroup = new THREE.Group();
      markerGroup.position.copy(destination);
      markerGroup.quaternion.setFromUnitVectors(
        new THREE.Vector3(0, 0, 1),
        destination.clone().normalize(),
      );
      const marker = new THREE.Mesh(
        new THREE.SphereGeometry(radius, 12, 8),
        new THREE.MeshBasicMaterial({ color: selected ? 0xe4f222 : 0xb1b9bd }),
      );
      const halo = new THREE.Mesh(
        new THREE.SphereGeometry(radius * 1.75, 12, 8),
        new THREE.MeshBasicMaterial({
          color: selected ? 0xe4f222 : 0x9ca7ad,
          transparent: true,
          opacity: selected ? 0.16 : 0.075,
          depthWrite: false,
        }),
      );
      if (connection.geo?.placement === "country_centroid") {
        halo.scale.y = 0.62;
        halo.rotation.z = Math.PI / 4;
      }
      const selectionTarget =
        selectedId && route.memberIds.includes(selectedId)
          ? selectedId
          : connection.id;
      marker.userData.connectionId = selectionTarget;
      marker.userData.connectionCount = route.count;
      marker.userData.worldPosition = destination;
      marker.userData.tooltip = `${connection.geo?.country_name ?? connection.domain ?? connection.key.remote.address} / AS${connection.asn?.number ?? "?"} / ${route.count} / ${connection.geo?.precision ?? "unknown"}`;
      state.markerIds.set(marker, selectionTarget);
      markerGroup.add(halo, marker);
      state.traffic.add(markerGroup);
      if (route.active && !reduceMotion) {
        const dot = new THREE.Mesh(
          new THREE.SphereGeometry(selected ? 0.009 : 0.006, 8, 6),
          new THREE.MeshBasicMaterial({
            color: selected ? 0xe4f222 : 0xd8dfe2,
            depthTest: true,
          }),
        );
        dot.position.copy(curve.getPoint((route.count % 10) / 10));
        state.traffic.add(dot);
        state.movingDots.push({
          mesh: dot,
          curve,
          phase: (route.count % 17) / 17,
          speed: 0.045 + (route.count % 7) * 0.004,
        });
      }
    }
    const homeGroup = new THREE.Group();
    homeGroup.position.copy(homePoint);
    homeGroup.add(
      new THREE.Mesh(
        new THREE.SphereGeometry(0.036, 12, 8),
        new THREE.MeshBasicMaterial({
          color: 0xe4f222,
          transparent: true,
          opacity: 0.1,
          depthWrite: false,
        }),
      ),
      new THREE.Mesh(
        new THREE.OctahedronGeometry(0.022, 0),
        new THREE.MeshBasicMaterial({ color: 0xe4f222 }),
      ),
    );
    state.traffic.add(homeGroup);

    if (selectedId && smartFocus && !reduceMotion) {
      const selected = routes.find((route) =>
        route.memberIds.includes(selectedId),
      );
      if (selected) {
        const target = new THREE.Vector3(
          ...spherePoint(selected.longitude, selected.latitude, 0.25),
        );
        state.controls.target.lerp(target, 0.72);
      }
    }
  }, [connections, home, reduceMotion, selectedId, smartFocus]);

  const resetCamera = () => {
    const state = sceneRef.current;
    if (!state) return;
    centerCameraOnSpain(state.camera);
    state.controls.target.set(0, 0, 0);
    state.controls.update();
  };

  const zoomCamera = (factor: number) => {
    const state = sceneRef.current;
    if (!state) return;
    const distance = Math.min(
      state.controls.maxDistance,
      Math.max(
        state.controls.minDistance,
        state.camera.position.length() * factor,
      ),
    );
    state.camera.position.setLength(distance);
    state.controls.update();
  };

  return (
    <div className="globe-scene-shell">
      <div
        aria-label={t(
          "Interactive globe showing filtered network destinations. Use the connection feed for a keyboard-accessible alternative.",
        )}
        className="globe-scene"
        data-testid="globe-view"
        ref={hostRef}
        role="img"
      />
      <div
        className="globe-scene__controls"
        role="toolbar"
        aria-label={t("Globe controls")}
      >
        <button
          aria-label={t("Zoom in")}
          onClick={() => zoomCamera(0.82)}
          type="button"
        >
          +
        </button>
        <button
          aria-label={t("Zoom out")}
          onClick={() => zoomCamera(1.22)}
          type="button"
        >
          −
        </button>
        <button onClick={resetCamera} type="button">
          {t("Reset camera")}
        </button>
      </div>
      {tooltip && (
        <div
          className="globe-scene__tooltip"
          style={{ left: tooltip.x, top: tooltip.y }}
          role="status"
        >
          {tooltip.text}
        </div>
      )}
      <span className="sr-only">
        {t("Rendering is capped at 120 routes with local geometry only.")}
      </span>
    </div>
  );
});

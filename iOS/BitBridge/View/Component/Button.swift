//
//  Button.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct PrimaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .frame(maxWidth: .infinity)
            .padding()
            .font(FontTheme.H2Heading)
            .foregroundColor(Theme.PrimaryText.color)
            .background(Theme.BluePrimary.color)
            .cornerRadius(.infinity)
    }
}

struct PrimaryGradientButton<S: ShapeStyle>: ButtonStyle {
    let gradient: S
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .frame(maxWidth: .infinity)
            .padding()
            .font(FontTheme.H2Heading)
            .foregroundColor(Theme.LightViolet.color)
            .background(gradient)
            .cornerRadius(.infinity)
            .overlay(
                RoundedRectangle(cornerRadius: .infinity)
                    .stroke(Theme.LightViolet.color.opacity(0.3), lineWidth: 1)
            )
            .shadow(color: Theme.LightViolet.color.opacity(0.2), radius: 15)
    }
}

struct PressableButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .opacity(configuration.isPressed ? 0.8 : 1.0)
            .animation(.easeInOut(duration: 0.2), value: configuration.isPressed)
            .environment(\.isPressed, configuration.isPressed)
    }
}

// Environment key to track button press state
private struct IsPressedKey: EnvironmentKey {
    static let defaultValue: Bool = false
}

extension EnvironmentValues {
    var isPressed: Bool {
        get { self[IsPressedKey.self] }
        set { self[IsPressedKey.self] = newValue }
    }
}

struct PrimaryButton: View {
    var body: some View {
        Button(action: {}) {
            Text("Button")
        }
        .buttonStyle(PrimaryGradientButton(gradient: Theme.primaryGradient))
    }
}

struct MoreOptionButton<T>: View {
    @Binding var state: Bool
    var item: T?
    @Binding var selectedItem: T?

    var body: some View {
        Button(action: {
            state = true
            selectedItem = item
        }) {
            ImageAsset.More.image
                .foregroundColor(Theme.PrimaryText.color)
                .scaleEffect(1.6)
        }
    }
}

struct UpgradePremiumButton: View {
    var body: some View {
        Button(action: {}) {
            Text("Upgrade to premium")
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .modifier(Label1())
                .foregroundStyle(Theme.PrimaryText.color.opacity(1))
                .background {
                    RoundedRectangle(cornerRadius: .infinity)
                        .fill(Theme.SeaTertiary.color.opacity(0.1))
                }
        }
    }
}

struct AddFileButton: View {
    var body: some View {
        Button(action: {}) {
            HStack {
                Text("Add new files")
                    .padding(.trailing, 4)
                Image(systemName: "plus")
                    .resizable()
                    .padding(.all, 8)
                    .frame(width: 30, height: 30)
            }
        }
        .background(Theme.BluePrimary.color)
        .foregroundColor(Theme.PrimaryText.color)
    }
}

struct Button_Preview: PreviewProvider {
    static var previews: some View {
        VStack {
            Spacer()
            PrimaryButton()
                .padding()
            Spacer()
        }
        .frame(width: .infinity, height: .infinity)
        .background(Theme.DarkViolet.color)
    }
}

struct CircleWaveEffect: ViewModifier {
    @State private var animationPhase: CGFloat = 0
    @State private var timer: Timer?
    @State private var isActive: Bool = false

    func body(content: Content) -> some View {
        content
            .visualEffect { content, proxy in
                return content
                    .colorEffect(
                        ShaderLibrary.circleWave(
                            .float2(proxy.size),
                            .color(Theme.BluePrimary.color),
                            .float(animationPhase * 0.4)
                        ),
                        isEnabled: isActive
                    )
            }
            .onAppear {
                startAnimation()
            }
            .onDisappear {
                stopAnimation()
            }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.willEnterForegroundNotification)) { _ in
                startAnimation()
            }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.didEnterBackgroundNotification)) { _ in
                stopAnimation()
            }
    }

    private func startAnimation() {
        guard timer == nil else { return }
        isActive = true
        timer = Timer.scheduledTimer(withTimeInterval: 1/14, repeats: true) { _ in
            animationPhase += 1/14
        }
    }

    private func stopAnimation() {
        timer?.invalidate()
        timer = nil
        isActive = false
    }
}

struct ShareButton: View {
    let width: CGFloat

    @State private var showShareModal = false
    @State private var shareModalContentHeight = CGFloat(0)
    @Environment(\.isPressed) private var isPressed

    var body: some View {
        return AnyView(ZStack {
            Circle()
                .fill(Theme.GreenSecondary.color.opacity(0.8))
                .modifier(CircleWaveEffect())
            Button(action: {
                Task {
                    showShareModal = true
                }
            }) {
                ImageAsset.SendEmpty.image
                    .rotationEffect(.degrees(-45))
                    .offset(x: 1, y: -1)
                    .frame(width: width * 0.3, height: width * 0.3)
                    .background(
                        Circle()
                            .foregroundStyle(Theme.BlackBase.color)
                    )
                    .clipShape(Circle())
            }
            .opacity(isPressed ? 0.8 : 1.0)
            .animation(.easeInOut(duration: 0.2), value: isPressed)
            .buttonStyle(PressableButtonStyle())
            .sheet(isPresented: $showShareModal) {
                ShareModal()
                    .presentationDetents([.height(shareModalContentHeight), .medium])
                    .presentationCornerRadius(36)
                    .presentationBackground(.clear)
                    .background {
                        GeometryReader { proxy in
                            Color.clear
                                .task {
                                    shareModalContentHeight = proxy.size.height
                                }
                        }
                    }
                    .background(Theme.BlackBase.color.opacity(0.3))
                    .background(StunningBackgroundGradientSecondary().opacity(0.2))
                    .background(.ultraThinMaterial)
                    .environment(\.colorScheme, .dark)
            }
        }
            .frame(width: width, height: width)
            .ignoresSafeArea()
        )
    }
}

#Preview("Premium button") {
    UpgradePremiumButton()
        .previewLayout(.sizeThatFits)
        .padding()
        .background(Theme.DarkViolet.color)
}

#Preview("Add file button") {
    AddFileButton()
        .previewLayout(.sizeThatFits)
        .padding()
        .background(Theme.DarkViolet.color)
}

#Preview("Share button") {
    ShareButton(width: 300)
}

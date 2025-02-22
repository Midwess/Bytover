//
//  Button.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

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

struct PrimaryButton: View {
    var body: some View {
        Button(action: {}) {
            Text("Button")
        }
        .buttonStyle(PrimaryGradientButton(gradient: Theme.primaryGradient))
    }
}

struct UpgradePremiumButton: View {
    var body: some View {
        Button(action: {}) {
            Text("Upgrade premium")
                .padding(.horizontal, 8)
                .padding(.vertical, 2)
                .foregroundStyle(Theme.textGradient)
                .modifier(Label4())
                .background {
                    RoundedRectangle(cornerRadius: .infinity)
                        .fill(Theme.LightViolet.color.opacity(0.1))
                        .stroke(Theme.PrimaryViolet.color.opacity(0.15), lineWidth: 1)
                }
        }
    }
}

struct AddFileButton: View {
    var body: some View {
        Button(action: {}) {
            Image(systemName: "plus")
                .resizable()
                .foregroundColor(Theme.LightViolet.color)
                .padding(.all, 8)
                .frame(width: 30, height: 30)
                .background(
                    RoundedRectangle(cornerRadius: .infinity)
                        .fill(Theme.LightViolet.color.opacity(0.1))
                        .stroke(Theme.PrimaryViolet.color.opacity(0.45), lineWidth: 1)
                )
        }
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

struct ButtonNavigation: View {
    var icon: String
    var label: String
    var body: some View {
        VStack {
            Image(systemName: icon)
                .resizable()
                .frame(width: 20, height: 20)
            Text(label)
                .modifier(Label4())
                .padding(.top, 4)
        }
    }
}

struct ShareButton: View {
    let width: CGFloat
    @State private var startTime = Date.now
    var body: some View {
        ZStack {
            Group {
                ZStack {
                    TimelineView(.animation) { timeline in
                        let elapsedTime = startTime.distance(to: timeline.date)
                        Circle()
                            .fill(Theme.PrimaryViolet.color.opacity(0.9))
                            .visualEffect { content, proxy in
                                content
                                    .colorEffect(
                                        ShaderLibrary.circleWave(
                                            .float2(proxy.size),
                                            .color(Theme.SecondaryVividViolet.color),
                                            .float(elapsedTime * 0.8)
                                        )
                                    )
                            }
                    }
                    Button(action: {}) {
                        Image(systemName: "paperplane")
                            .foregroundColor(Theme.LightPrimaryViolet.color)
                            .fontWeight(.medium)
                            .font(.title2)
                    }
                    .frame(width: width * 0.49, height: width * 0.49)
                    .background(Theme.circlePrimaryGradient)
                    .clipShape(Circle())
                }
            }
        }
        .frame(width: width, height: width)
        .ignoresSafeArea()
    }
}

#Preview("Bottom navigation") {
    ButtonNavigation(icon: "plus", label: "plus")
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

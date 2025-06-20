//
//  Toast.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 27/4/25.
//

import SwiftUI
import Foundation

struct Alert: View {
    @EnvironmentObject private var core: Core
    @State var toastMessage: String?
    @State var opacity: Double = 0

    @State var isShowingAlert = false
    @State var isShowingConfirmation = false

    var body: some View {
        Text(core.toastMessage.value ?? "")
            .font(FontTheme.Body2)
            .foregroundStyle(Theme.PrimaryText.color)
            .padding()
            .background(Capsule().foregroundStyle(Theme.BlackBase.color).blur(radius: 10))
            .overlay(
                Capsule()
                    .stroke(.gray.opacity(0.8), lineWidth: 0.3)
            )
            .opacity(opacity)
            .animation(.easeInOut(duration: 0.2), value: opacity)
            .onReceive(core.toastMessage, perform: { value in
                if value?.isEmpty ?? true {
                    return
                }

                toastMessage = value
                opacity = 1

                DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                    toastMessage = nil
                    opacity = 0
                }
            })
            .alert(core.alert.value?.0.message ?? "", isPresented: $isShowingAlert) {
                Button(core.alert.value?.0.affirmative ?? "") {
                    core.alert.value?.1.resolve(true)
                }
            }
            .confirmationDialog(core.alert.value?.0.message ?? "", isPresented: $isShowingConfirmation, titleVisibility: .visible) {
                Button(core.alert.value?.0.affirmative ?? "Yes", role: .destructive) {
                    core.alert.value?.1.resolve(true)
                }

                Button(core.alert.value?.0.negative ?? "Cancel", role: .cancel) {
                    core.alert.value?.1.resolve(false)
                }
            }
            .onReceive(core.alert, perform: { value in
                self.isShowingAlert = value != nil && value?.0.negative == nil
                self.isShowingConfirmation = value != nil && value?.0.affirmative != nil && value?.0.negative != nil
            })
    }
}

#Preview {
    var core = CoreMock.empty() as Core
    Alert()
        .environmentObject(core)
}
